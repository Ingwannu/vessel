//! When the first-run walkthrough is on screen, and which step.
//!
//! [Decision Log]
//! - Purpose: ADR 0071. Paseo's setup is one decision per launch and one
//!   completion record; the step list is not stored.
//! - Constraint: this crate does no I/O and holds no UI type. The clock,
//!   the prefs file and the node are the caller's.
//! - Choice: a phase the caller feeds with a record and the machines'
//!   project facts. A visit number makes a late action a no-op.
//! - Why not a stored step: Paseo does not store one, so a relaunch of an
//!   unfinished setup starts again at the theme.
//! - Cost: quitting mid-setup forgets the step. An unreadable prefs file
//!   hides setup, because opening it would trap someone who already uses
//!   the app, and the file must not be overwritten.

mod record;

pub use record::{BadTime, CompletedAt, SetupRecord};

use record::Loaded;

/// Whether this app is the phone being paired.
///
/// A narrow window is not a phone. This module does not read the window;
/// the caller knows which binary it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Client {
    /// The desktop app, at any width. The phone step is included.
    Desktop,
    /// This app is the phone. The phone step is omitted.
    Phone,
}

/// The steps, in Paseo's order, with pairing where the daemon stood.
///
/// Theme needs no machine. The phone step is last and only on
/// [`Client::Desktop`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The list Settings › Appearance draws. Applied by the caller at the pick.
    Theme,
    /// Pair a node. Continuing does not require one.
    Machine,
    /// What the node's checks already say. Continuing does not require a sign-in.
    Agents,
    /// Add a folder.
    Project,
    /// The tools switch. Skills and the CLI are not offered.
    Orchestration,
    /// Pair a phone, on the desktop app only.
    Phone,
}

impl Step {
    /// Desktop order, phone step included.
    const DESKTOP: [Self; 6] = [
        Self::Theme,
        Self::Machine,
        Self::Agents,
        Self::Project,
        Self::Orchestration,
        Self::Phone,
    ];

    /// Phone-app order. The phone step is the thing this client already is.
    const PHONE: [Self; 5] = [
        Self::Theme,
        Self::Machine,
        Self::Agents,
        Self::Project,
        Self::Orchestration,
    ];

    /// The list `client` walks. Fixed for a launch once setup opens.
    #[must_use]
    pub const fn list(client: Client) -> &'static [Self] {
        match client {
            Client::Desktop => &Self::DESKTOP,
            Client::Phone => &Self::PHONE,
        }
    }

    /// The step before this one in `client`'s list.
    #[must_use]
    pub fn previous(self, client: Client) -> Option<Self> {
        neighbor(self, client, Way::Back)
    }

    /// The step after this one. [`None`](Option::None) on the last: that move finishes.
    #[must_use]
    pub fn following(self, client: Client) -> Option<Self> {
        neighbor(self, client, Way::Forward)
    }
}

/// Which way [`neighbor`] walks the client's list.
#[derive(Clone, Copy)]
enum Way {
    Forward,
    Back,
}

/// The tools switch starts on, as Paseo's orchestration does.
///
/// The screen offers it only when ADR 0065's key is present and this
/// device holds admin, and it does not move on until that write succeeds.
/// This type does not perform the write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolsChoice {
    /// Agents on that machine get the tools.
    #[default]
    On,
    /// Leave them off.
    Off,
}

fn neighbor(step: Step, client: Client, way: Way) -> Option<Step> {
    let list = Step::list(client);
    let index = list.iter().position(|item| *item == step)?;
    // checked: a wrapped index would name a different step. The lists are
    // short; the check is the rule, not a bound this list reaches.
    let next = match way {
        Way::Forward => index.checked_add(1)?,
        Way::Back => index.checked_sub(1)?,
    };
    list.get(next).copied()
}

/// One stay on a step.
///
/// The caller passes this back to [`Onboarding::next`]. A number from a
/// stay the person has left does not move the step they are on now, and
/// does not finish a setup they closed and opened again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visit(u64);

/// Whether this client has finished setup before.
///
/// Missing and completed are different facts. An unreadable prefs file is
/// [`Onboarding::note_file_unreadable`], not a third value here, because
/// the file is the caller's and this value is only what it read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Completion {
    /// No record. Setup may still open.
    Pending,
    /// Finished or skipped on this client.
    Completed(CompletedAt),
}

/// What the caller knows about paired machines, for the one decision.
///
/// Project lists that are still being read must not count as empty: an
/// empty list opens setup, and a list not yet read is not empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineFacts {
    /// Nothing is paired. There is no daemon to wait for, so this can open setup.
    None,
    /// At least one machine, and no project list has been read yet.
    ProjectsUnknown,
    /// Every paired machine's list has been read.
    Projects(Projects),
}

/// Whether any paired machine already has a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Projects {
    /// Every known list is empty.
    None,
    /// At least one project. Setup stays closed for this launch.
    Some,
}

/// Where the walkthrough is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// The record, or the project lists, are not known yet.
    Deciding,
    /// Not on screen, until [`Onboarding::reopen`].
    Hidden(Hidden),
    /// On screen at one step.
    Open(Open),
}

/// Why it is not on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hidden {
    /// Finished, skipped, or the prefs file could not be read.
    Finished,
    /// A paired machine already had a project when this launch decided.
    AlreadySetUp,
}

/// The step on screen, and the visit an action must name to move on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Open {
    /// Which step.
    pub step: Step,
    /// The stay. Changes when the step changes.
    pub visit: Visit,
    /// Which list this opening is walking. A later note does not change it.
    pub client: Client,
}

/// What moving on did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Advance {
    /// Another step is showing.
    Moved,
    /// The last step finished setup. The caller writes the record.
    Finished(Remember),
    /// Not open, a visit already left, or the visit counter cannot grow.
    Ignored,
}

/// The completion record to write. This module does not write it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Remember {
    /// When the caller finished or skipped.
    pub at: CompletedAt,
}

/// The walkthrough's phase for one launch.
///
/// The first note of the record sticks. Machine facts may arrive again
/// until the phase leaves [`Phase::Deciding`]. After that they do not
/// open or close it: a project added during setup leaves it open, and a
/// project removed afterwards leaves it closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Onboarding {
    phase: Phase,
    record: Loaded,
    machines: Option<MachineFacts>,
    /// Noted while deciding, and again by [`Self::reopen`].
    client: Option<Client>,
    /// The next visit to issue. It only grows, so a stale visit cannot
    /// collide with a later stay of this same model.
    next_visit: u64,
}

impl Onboarding {
    /// Nothing noted yet. The caller has not read the record or the machines.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            phase: Phase::Deciding,
            record: Loaded::Waiting,
            machines: None,
            client: None,
            next_visit: 1,
        }
    }

    /// Where it is. The caller draws this; nothing here notifies a window.
    #[must_use]
    pub const fn phase(&self) -> &Phase {
        &self.phase
    }

    /// The record, once. A second call while still deciding is ignored,
    /// so a late read cannot override the one this launch decided from.
    pub fn note_completion(&mut self, completion: Completion) {
        let loaded = match completion {
            Completion::Pending => Loaded::Pending,
            Completion::Completed(_) => Loaded::Completed,
        };
        self.note(loaded);
    }

    /// The prefs file could not be read. Hide setup and do not ask for a
    /// write: the caller's save already refuses an unreadable file.
    pub fn note_file_unreadable(&mut self) {
        self.note(Loaded::Unreadable);
    }

    /// Paired machines, or the lack of them. Replaces an earlier note
    /// only while still deciding, so "still loading" can become a list.
    pub fn note_machines(&mut self, facts: MachineFacts) {
        if !matches!(self.phase, Phase::Deciding) {
            return;
        }
        self.machines = Some(facts);
        self.decide();
    }

    /// Desktop or phone. Replaces an earlier note only while still deciding,
    /// so the list cannot lose the phone step after it has opened.
    pub fn note_client(&mut self, client: Client) {
        if !matches!(self.phase, Phase::Deciding) {
            return;
        }
        self.client = Some(client);
        self.decide();
    }

    /// Show the theme again, on `client`'s list, even if this launch hid setup.
    ///
    /// Does not clear a record the caller has stored. A counter that cannot
    /// grow leaves the phase as it was. `client` is the list for this opening,
    /// as Paseo passes `includePhoneStep` at reopen.
    pub fn reopen(&mut self, client: Client) {
        self.client = Some(client);
        self.enter(Step::Theme);
    }

    /// Move on from the stay the caller saw.
    ///
    /// An add or a tools write still in flight is the screen's. It calls
    /// this only after that write has succeeded, and it passes the visit
    /// from the stay where the write started, so a write that finishes
    /// after the person has moved does not move them again.
    ///
    /// `at` is written only on the last step. The model does not read the
    /// clock, so the caller passes the time on every move.
    #[must_use]
    pub fn next(&mut self, visit: Visit, at: CompletedAt) -> Advance {
        let (step, client) = match &self.phase {
            Phase::Open(open) if open.visit == visit => (open.step, open.client),
            Phase::Open(_) | Phase::Deciding | Phase::Hidden(_) => return Advance::Ignored,
        };
        match step.following(client) {
            Some(step) => {
                let before = self.phase;
                self.enter(step);
                if self.phase == before {
                    Advance::Ignored
                } else {
                    Advance::Moved
                }
            }
            None => Advance::Finished(self.complete(at)),
        }
    }

    /// The previous step. On the theme, or while closed, this does nothing.
    pub fn back(&mut self) {
        let Some((step, client)) = self.current() else {
            return;
        };
        let Some(step) = step.previous(client) else {
            return;
        };
        // A counter that cannot grow leaves the step where it is.
        self.enter(step);
    }

    /// Jump to a step in this opening's list, including one not yet reached.
    /// The step already showing is left as it is, visit included. A step the
    /// list does not have (the phone step on a phone) does nothing.
    pub fn go_to(&mut self, step: Step) {
        let Some((current, client)) = self.current() else {
            return;
        };
        if current == step || !Step::list(client).contains(&step) {
            return;
        }
        self.enter(step);
    }

    /// Close setup and ask the caller to write the record.
    ///
    /// This is skip, the sheet's close, and the end of [`Self::next`] on
    /// the last step. It hides setup even if it was not open.
    #[must_use]
    pub fn complete(&mut self, at: CompletedAt) -> Remember {
        self.phase = Phase::Hidden(Hidden::Finished);
        Remember { at }
    }

    fn note(&mut self, loaded: Loaded) {
        if !matches!(self.phase, Phase::Deciding) || !matches!(self.record, Loaded::Waiting) {
            return;
        }
        self.record = loaded;
        self.decide();
    }

    fn decide(&mut self) {
        if !matches!(self.phase, Phase::Deciding) {
            return;
        }
        match self.record {
            Loaded::Waiting => {}
            Loaded::Completed | Loaded::Unreadable => {
                self.phase = Phase::Hidden(Hidden::Finished);
            }
            Loaded::Pending => match (self.client, self.machines) {
                (None, _) | (_, None | Some(MachineFacts::ProjectsUnknown)) => {}
                (Some(_), Some(MachineFacts::None | MachineFacts::Projects(Projects::None))) => {
                    self.enter(Step::Theme);
                }
                (Some(_), Some(MachineFacts::Projects(Projects::Some))) => {
                    self.phase = Phase::Hidden(Hidden::AlreadySetUp);
                }
            },
        }
    }

    fn current(&self) -> Option<(Step, Client)> {
        match self.phase {
            Phase::Open(open) => Some((open.step, open.client)),
            Phase::Deciding | Phase::Hidden(_) => None,
        }
    }

    /// Issue the next visit and show `step`. When the client is unknown or
    /// the counter cannot grow, the phase is left unchanged. [`Self::reopen`]
    /// sets the client first.
    fn enter(&mut self, step: Step) {
        let Some(client) = self.client else {
            return;
        };
        let Some(visit) = self.take_visit() else {
            return;
        };
        self.phase = Phase::Open(Open {
            step,
            visit,
            client,
        });
    }

    fn take_visit(&mut self) -> Option<Visit> {
        let issued = self.next_visit;
        // checked_add: wrapping would reuse a visit already left, and a
        // late action from that stay would move the step on screen now.
        self.next_visit = issued.checked_add(1)?;
        Some(Visit(issued))
    }
}

impl Default for Onboarding {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
