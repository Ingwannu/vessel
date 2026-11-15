//! Empty workspace, chat, blank shell and profile launch remain distinct intents.
//!
//! The reference empty/terminal helpers never create a chat as a side effect.
//! Profiles use literal argv substitution; only blank-shell text becomes typed
//! shell input. Resolving a removed selection happens visibly before submit;
//! a removal racing preparation is an explicit refusal, never another target.

use std::{fmt, sync::Arc};
use vessel_proto::terminal_profiles::{Id, ProfileError, Profiles, Prompt, Resolved};

/// A persisted picker choice; the default shell is not a configured profile id.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Launch {
    /// No prompt/payload means an empty workspace rather than an empty agent.
    #[default]
    Chat,
    /// The node's configured shell, followed by optional typed input.
    BlankTerminal,
    /// An administrator's current literal command/argv template.
    Profile(Id),
}

/// File and context owners report presence without transferring their payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadPresence {
    /// No attached file or context accompanies the text.
    Empty,
    /// At least one file/context must be delivered to a chat.
    Present,
}

/// A validated launch plan contains no RPC or filesystem effect.
#[derive(Clone, PartialEq, Eq)]
pub enum LaunchPlan {
    /// Create the independent workspace without an initial resource.
    EmptyWorkspace,
    /// The submission owner retains files/context; text keeps its whitespace.
    Chat(Arc<str>),
    /// A blank shell receives trimmed input plus CR only after registration.
    BlankTerminal(Option<ShellInput>),
    /// A profile starts its resolved argv directly, never by typing shell text.
    Profile {
        /// Resolve the same profile identity at authoritative admission.
        id: Id,
        /// Command and argv preserve literal argument boundaries.
        command: Resolved,
        /// Empty for a profile without a prompt sentinel.
        naming_prompt: Arc<str>,
    },
}

impl fmt::Debug for LaunchPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::EmptyWorkspace => "EmptyWorkspace",
            Self::Chat(_) => "Chat([redacted])",
            Self::BlankTerminal(_) => "BlankTerminal([redacted])",
            Self::Profile { .. } => "Profile([redacted])",
        })
    }
}

/// Bounded blank-shell input is sent through terminal input, not an argv join.
#[derive(Clone, PartialEq, Eq)]
pub struct ShellInput(Prompt);
impl ShellInput {
    /// Reserve the final CR inside the existing terminal input call's byte cap.
    pub const MAX_BYTES: usize = vessel_proto::terminals::SendTerminal::MAX_BYTES.saturating_sub(1);

    /// The one CR completes the person's trimmed command at their shell prompt.
    #[must_use]
    pub fn terminal_data(&self) -> String {
        format!("{}\r", self.0.as_str())
    }
}
impl fmt::Debug for ShellInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ShellInput([redacted])")
    }
}

/// Do not discard chat payload just because the selected renderer changed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LaunchProblem {
    /// The catalog changed after the form reconciled its selection.
    #[error("the selected terminal profile is no longer configured")]
    MissingProfile,
    /// Keep the original validated template/input refusal without raw text.
    #[error(transparent)]
    Profile(#[from] ProfileError),
    /// Terminals cannot silently consume or discard chat attachments/context.
    #[error("attached chat payload requires a chat target")]
    ChatPayload,
}

impl Launch {
    /// Reconcile preferences before presenting the form, as Paseo target.ts does.
    #[must_use]
    pub fn reconciled(&self, profiles: &Profiles) -> Self {
        match self {
            Self::Profile(id) if profiles.get(id).is_none() => Self::Chat,
            Self::Chat | Self::BlankTerminal | Self::Profile(_) => self.clone(),
        }
    }

    /// Plan one accepted submission without creating an empty chat or shell argv.
    /// # Errors
    /// A removed profile, bounded input failure, or non-chat payload.
    pub fn prepare(
        &self,
        text: &str,
        payload: PayloadPresence,
        profiles: &Profiles,
    ) -> Result<LaunchPlan, LaunchProblem> {
        match self {
            Self::Chat => Ok(
                if text.trim().is_empty() && payload == PayloadPresence::Empty {
                    LaunchPlan::EmptyWorkspace
                } else {
                    LaunchPlan::Chat(Arc::from(text))
                },
            ),
            Self::BlankTerminal => {
                if payload == PayloadPresence::Present {
                    return Err(LaunchProblem::ChatPayload);
                }
                let trimmed = text.trim();
                if trimmed.len() > ShellInput::MAX_BYTES {
                    return Err(ProfileError::TooLong {
                        kind: vessel_proto::terminal_profiles::TextKind::Prompt,
                        max: ShellInput::MAX_BYTES,
                    }
                    .into());
                }
                let input = (!trimmed.is_empty())
                    .then(|| Prompt::parse(trimmed).map(ShellInput))
                    .transpose()?;
                Ok(LaunchPlan::BlankTerminal(input))
            }
            Self::Profile(id) => {
                if payload == PayloadPresence::Present {
                    return Err(LaunchProblem::ChatPayload);
                }
                let profile = profiles.get(id).ok_or(LaunchProblem::MissingProfile)?;
                let prompt = Prompt::parse(text)?;
                let command = profile.resolve(&prompt)?;
                let naming_prompt = Arc::from(if profile.takes_prompt() { text } else { "" });
                Ok(LaunchPlan::Profile {
                    id: id.clone(),
                    command,
                    naming_prompt,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests;
