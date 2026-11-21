//! A machine's terminal profiles as the app lists and edits them (ADR 0117,
//! Paseo's `TerminalProfilesSection`, `screens/settings/host-page.tsx`
//! 1311-1665, and its edit modal). No toolkit or I/O.
//!
//! [Decision Log]
//! - Purpose: New tab's profile rows and Settings › Terminals read one list
//!   per machine; Settings changes it.
//! - Reference: the list is the daemon's (absent: the shipped four). Every
//!   change saves the whole list. Name and command are required after
//!   trimming. Arguments split on whitespace, with no quoting
//!   (`parseArgsString`). An edit keeps the profile's id and icon. Ids are
//!   `profile_<base36 time>_<base36 random>` (`generateProfileId`).
//! - Choice: one `Listing` a machine, numbered by tickets so only the
//!   latest answer counts. A save keeps the list shown until the node
//!   answers. A refusal is kept beside the list, which stays as it was.
//! - Cost: two people editing one machine's list: the later save wins, as in
//!   Paseo, whose config patch replaces the list too.

use vessel_proto::ErrorCode;
use vessel_proto::terminal_profiles::{
    Argument, Id, Name, Profile, ProfileError, Profiles, Program,
};

use crate::devices::Ticket;

/// Why a machine's list could not be read or kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    /// No live connection to the machine.
    #[error("the machine is not connected")]
    Offline,
    /// Its node predates terminal profiles (below protocol 45).
    #[error("the machine's node is too old for terminal profiles")]
    TooOld,
    /// The node refused the call, with this code (`forbidden`: this device
    /// may read but not administer).
    #[error("the node refused the call ({0:?})")]
    Refused(ErrorCode),
}

/// One machine's list as the app knows it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Listing {
    ticket: Option<Ticket>,
    read: Option<Result<Profiles, Problem>>,
    saving: bool,
    refused: Option<Problem>,
}

impl Listing {
    /// A read was sent with `ticket`.
    pub fn reading(&mut self, ticket: Ticket) {
        self.ticket = Some(ticket);
    }

    /// A save of the whole list was sent with `ticket`; refused while
    /// another waits, as Paseo's controls wait.
    pub fn saving_with(&mut self, ticket: Ticket) -> bool {
        if self.saving {
            return false;
        }
        self.ticket = Some(ticket);
        self.saving = true;
        self.refused = None;
        true
    }

    /// An answer arrived, kept only when it is the latest request's. For a
    /// save, its outcome (`Some`): the list on success, the refusal kept
    /// beside the list otherwise.
    pub fn answered(
        &mut self,
        ticket: Ticket,
        result: Result<Profiles, Problem>,
    ) -> Option<Result<(), Problem>> {
        if self.ticket != Some(ticket) {
            return None;
        }
        if !self.saving {
            self.read = Some(result);
            return None;
        }
        self.saving = false;
        Some(match result {
            Ok(profiles) => {
                self.read = Some(Ok(profiles));
                Ok(())
            }
            Err(problem) => {
                self.refused = Some(problem);
                Err(problem)
            }
        })
    }

    /// The list, once read.
    #[must_use]
    pub fn profiles(&self) -> Option<&Profiles> {
        self.read.as_ref().and_then(|read| read.as_ref().ok())
    }

    /// Why it could not be read.
    #[must_use]
    pub fn problem(&self) -> Option<Problem> {
        self.read
            .as_ref()
            .and_then(|read| read.as_ref().err().copied())
    }

    /// Whether a read or a save waits for its answer.
    #[must_use]
    pub const fn waiting(&self) -> bool {
        self.saving || (self.ticket.is_some() && self.read.is_none())
    }

    /// Whether a save waits (the row buttons and Add wait meanwhile).
    #[must_use]
    pub const fn saving(&self) -> bool {
        self.saving
    }

    /// The last save the node refused.
    #[must_use]
    pub const fn refused(&self) -> Option<Problem> {
        self.refused
    }
}

/// Which way a row moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// One place up (Paseo's Move up).
    Up,
    /// One place down.
    Down,
}

/// The list with the row at `index` moved one place; `None` at an end.
#[must_use]
pub fn moved(list: &Profiles, index: usize, step: Step) -> Option<Profiles> {
    let other = match step {
        Step::Up => index.checked_sub(1)?,
        Step::Down => index.checked_add(1)?,
    };
    let mut rows = list.entries().to_vec();
    if other >= rows.len() || index >= rows.len() {
        return None;
    }
    rows.swap(index, other);
    Profiles::new(rows).ok()
}

/// The list without profile `id`.
#[must_use]
pub fn removed(list: &Profiles, id: &Id) -> Profiles {
    let rows = list
        .entries()
        .iter()
        .filter(|profile| profile.id() != id)
        .cloned()
        .collect();
    Profiles::new(rows).unwrap_or_else(|_| list.clone())
}

/// What the edit dialog holds, as typed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Draft {
    /// The Name field.
    pub name: String,
    /// The Command field.
    pub command: String,
    /// The Arguments field.
    pub args: String,
}

/// What is wrong with a draft (Paseo's `nameRequired`, `commandRequired`),
/// or the bound it breaks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DraftErrors {
    /// The name is empty after trimming.
    pub name: bool,
    /// The command is empty after trimming.
    pub command: bool,
    /// A field or the list breaks a bound (ADR 0117's limits).
    pub invalid: Option<ProfileError>,
}

/// Which row the dialog changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A new row, last, with this id.
    Add(Id),
    /// The row with this id, keeping its id and icon.
    Edit(Id),
}

/// Paseo's `parseArgsString`: whitespace splits, no quoting.
#[must_use]
pub fn parse_args(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

/// The list with `draft` saved at `target`, or what is wrong with it.
///
/// # Errors
/// A required field is empty, or a value or the list breaks a bound.
pub fn with_draft(
    list: &Profiles,
    target: &Target,
    draft: &Draft,
) -> Result<Profiles, DraftErrors> {
    let name = draft.name.trim();
    let command = draft.command.trim();
    let mut errors = DraftErrors {
        name: name.is_empty(),
        command: command.is_empty(),
        invalid: None,
    };
    if errors.name || errors.command {
        return Err(errors);
    }
    let built = build(list, target, name, command, &draft.args);
    built.map_err(|error| {
        errors.invalid = Some(error);
        errors
    })
}

fn build(
    list: &Profiles,
    target: &Target,
    name: &str,
    command: &str,
    args: &str,
) -> Result<Profiles, ProfileError> {
    let args = parse_args(args)
        .into_iter()
        .map(Argument::parse)
        .collect::<Result<Vec<_>, _>>()?;
    let (id, icon) = match target {
        Target::Add(id) => (id.clone(), None),
        Target::Edit(id) => (
            id.clone(),
            list.get(id).and_then(|profile| profile.icon().cloned()),
        ),
    };
    let profile = Profile::new(id, Name::parse(name)?, Program::parse(command)?, args, icon)?;
    let mut rows = list.entries().to_vec();
    match target {
        Target::Add(_) => rows.push(profile),
        Target::Edit(id) => {
            for row in &mut rows {
                if row.id() == id {
                    *row = profile.clone();
                }
            }
        }
    }
    Profiles::new(rows)
}

/// The fields a row's Edit opens with: its name, command and arguments
/// joined by spaces (Paseo's `args.join(" ")`).
#[must_use]
pub fn draft_of(profile: &Profile) -> Draft {
    Draft {
        name: profile.name().as_str().to_owned(),
        command: profile.command().as_str().to_owned(),
        args: args_text(profile),
    }
}

fn args_text(profile: &Profile) -> String {
    profile
        .args()
        .iter()
        .map(Argument::as_str)
        .collect::<Vec<_>>()
        .join(" ")
}

/// What a row shows under its name: the command and its arguments (Paseo's
/// `commandText`).
#[must_use]
pub fn command_line(profile: &Profile) -> String {
    let args = args_text(profile);
    if args.is_empty() {
        profile.command().as_str().to_owned()
    } else {
        format!("{} {args}", profile.command().as_str())
    }
}

fn base36(mut value: u64) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();
    loop {
        let digit = value.checked_rem(36).unwrap_or_default();
        out.push(
            DIGITS
                .get(usize::try_from(digit).unwrap_or_default())
                .copied()
                .unwrap_or(b'0'),
        );
        value = value.checked_div(36).unwrap_or_default();
        if value == 0 {
            break;
        }
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// A new profile's id, Paseo's shape: `profile_<base36 ms>_<base36 random>`.
///
/// # Errors
/// Never in practice: both parts are base-36 digits within the id's bound.
pub fn new_id(now_ms: u64, random: u64) -> Result<Id, ProfileError> {
    Id::parse(format!("profile_{}_{}", base36(now_ms), base36(random)))
}
