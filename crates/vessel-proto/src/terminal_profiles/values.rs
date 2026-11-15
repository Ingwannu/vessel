//! Ordered profiles and substitution preserve argv boundaries without a shell.

use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::{
    Argument, Icon, Id, MAX_ARGUMENTS, MAX_COMMAND_BYTES, MAX_PROFILES, Name, PROMPT_SENTINEL,
    ProfileError, Program, Prompt, TextKind,
};

/// A validated administrator-defined terminal launch template.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ProfileWire", into = "ProfileWire")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Profile {
    id: Id,
    name: Name,
    command: Program,
    args: Vec<Argument>,
    icon: Option<Icon>,
}

#[derive(Serialize, Deserialize)]
struct ProfileWire {
    id: Id,
    name: Name,
    command: Program,
    #[serde(default)]
    args: Vec<Argument>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    icon: Option<Icon>,
}

impl Profile {
    /// Build only after all fields and aggregate argv bounds are checked.
    /// # Errors
    /// The argv count or combined command/argument size exceeds its bound.
    pub fn new(
        id: Id,
        name: Name,
        command: Program,
        args: Vec<Argument>,
        icon: Option<Icon>,
    ) -> Result<Self, ProfileError> {
        check_command(&command, &args)?;
        Ok(Self {
            id,
            name,
            command,
            args,
            icon,
        })
    }

    /// Stable identity for picker selection and profile lookup.
    #[must_use]
    pub const fn id(&self) -> &Id {
        &self.id
    }

    /// The person's label rather than a program-derived guess.
    #[must_use]
    pub const fn name(&self) -> &Name {
        &self.name
    }

    /// Catalog identity used only for drawing.
    #[must_use]
    pub const fn icon(&self) -> Option<&Icon> {
        self.icon.as_ref()
    }

    /// Profiles without a placeholder launch read-only, as the reference does.
    #[must_use]
    pub fn takes_prompt(&self) -> bool {
        self.command.as_str().contains(PROMPT_SENTINEL)
            || self
                .args
                .iter()
                .any(|arg| arg.as_str().contains(PROMPT_SENTINEL))
    }

    /// Substitute literal text into every occurrence, preserving each argv entry.
    /// # Errors
    /// The expanded command/arguments exceed bounds or produce an empty program.
    pub fn resolve(&self, prompt: &Prompt) -> Result<Resolved, ProfileError> {
        let text = prompt.as_str();
        let command = Program::parse(substitute(
            self.command.as_str(),
            text,
            Program::MAX_BYTES,
            TextKind::Program,
        )?)?;
        let mut args = Vec::new();
        for arg in &self.args {
            let arg = arg.as_str();
            if text.is_empty() && prompt_only(arg) {
                continue;
            }
            args.push(Argument::parse(substitute(
                arg,
                text,
                Argument::MAX_BYTES,
                TextKind::Argument,
            )?)?);
        }
        check_command(&command, &args)?;
        Ok(Resolved { command, args })
    }
}

fn substitute(
    template: &str,
    prompt: &str,
    max: usize,
    kind: TextKind,
) -> Result<String, ProfileError> {
    let count = template.matches(PROMPT_SENTINEL).count();
    // Check before allocating: a small repeated-placeholder template must not
    // transiently allocate an unbounded multiple of a permitted prompt.
    let expanded = count
        .checked_mul(PROMPT_SENTINEL.len())
        .and_then(|removed| template.len().checked_sub(removed))
        .and_then(|kept| {
            count
                .checked_mul(prompt.len())
                .and_then(|added| kept.checked_add(added))
        });
    if expanded.is_none_or(|size| size > max) {
        return Err(ProfileError::TooLong { kind, max });
    }
    Ok(template.replace(PROMPT_SENTINEL, prompt))
}

fn prompt_only(arg: &str) -> bool {
    arg == PROMPT_SENTINEL
        || arg
            .split_once('=')
            .is_some_and(|(option, value)| !option.is_empty() && value == PROMPT_SENTINEL)
}

fn check_command(command: &Program, args: &[Argument]) -> Result<(), ProfileError> {
    if args.len() > MAX_ARGUMENTS {
        return Err(ProfileError::TooManyArguments);
    }
    let total = args.iter().try_fold(command.as_str().len(), |count, arg| {
        count.checked_add(arg.as_str().len())
    });
    if total.is_none_or(|total| total > MAX_COMMAND_BYTES) {
        return Err(ProfileError::CommandTooLarge);
    }
    Ok(())
}

impl TryFrom<ProfileWire> for Profile {
    type Error = ProfileError;
    fn try_from(wire: ProfileWire) -> Result<Self, Self::Error> {
        Self::new(wire.id, wire.name, wire.command, wire.args, wire.icon)
    }
}

impl From<Profile> for ProfileWire {
    fn from(profile: Profile) -> Self {
        Self {
            id: profile.id,
            name: profile.name,
            command: profile.command,
            args: profile.args,
            icon: profile.icon,
        }
    }
}

impl fmt::Debug for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Profile")
            .field("id", &self.id)
            .field("arguments", &self.args.len())
            .finish_non_exhaustive()
    }
}

/// Expanded command ready for the existing process-launch boundary.
#[derive(Clone, PartialEq, Eq)]
pub struct Resolved {
    command: Program,
    args: Vec<Argument>,
}

impl Resolved {
    /// A literal program/path, not shell source.
    #[must_use]
    pub const fn command(&self) -> &Program {
        &self.command
    }

    /// Preserve OS argument boundaries, including spaces and metacharacters.
    #[must_use]
    pub fn args(&self) -> &[Argument] {
        &self.args
    }
}

impl fmt::Debug for Resolved {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Resolved")
            .field("arguments", &self.args.len())
            .finish_non_exhaustive()
    }
}

/// A bounded ordered profile set; an explicit empty list disables all profiles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<Profile>", into = "Vec<Profile>")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "Profile[]"))]
pub struct Profiles(Vec<Profile>);

impl Profiles {
    /// Preserve configured order while refusing ambiguous duplicate identities.
    /// # Errors
    /// Too many profiles or the same id appears more than once.
    pub fn new(profiles: Vec<Profile>) -> Result<Self, ProfileError> {
        if profiles.len() > MAX_PROFILES {
            return Err(ProfileError::TooManyProfiles);
        }
        let mut seen = HashSet::new();
        if profiles.iter().any(|profile| !seen.insert(profile.id())) {
            return Err(ProfileError::DuplicateId);
        }
        Ok(Self(profiles))
    }

    /// Picker and configuration rows, in the administrator's order.
    #[must_use]
    pub fn entries(&self) -> &[Profile] {
        &self.0
    }

    /// Resolve by identity; labels and program paths are not identities.
    #[must_use]
    pub fn get(&self, id: &Id) -> Option<&Profile> {
        self.0.iter().find(|profile| profile.id() == id)
    }

    /// Reference defaults, used only when configuration is absent.
    /// # Errors
    /// A shipped constant no longer satisfies profile validation.
    pub fn shipped() -> Result<Self, ProfileError> {
        let defaults = [
            (
                "claude",
                "Claude Code",
                "claude",
                PROMPT_SENTINEL.to_owned(),
            ),
            ("codex", "Codex", "codex", PROMPT_SENTINEL.to_owned()),
            (
                "opencode",
                "OpenCode",
                "opencode",
                format!("--prompt={PROMPT_SENTINEL}"),
            ),
            ("pi", "Pi", "pi", PROMPT_SENTINEL.to_owned()),
        ];
        let mut profiles = Vec::new();
        for (id, name, command, arg) in defaults {
            profiles.push(Profile::new(
                Id::parse(id)?,
                Name::parse(name)?,
                Program::parse(command)?,
                vec![Argument::parse(arg)?],
                Some(Icon::parse(id)?),
            )?);
        }
        Self::new(profiles)
    }
}

impl TryFrom<Vec<Profile>> for Profiles {
    type Error = ProfileError;
    fn try_from(value: Vec<Profile>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Profiles> for Vec<Profile> {
    fn from(value: Profiles) -> Self {
        value.0
    }
}
