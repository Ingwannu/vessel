//! Pure staged preparation over the existing shell spec and literal profiles.
//! It grants no spawn/root authority and does not dispatch a process or input.
use crate::terminals::{Shell, shell_spec};
use std::ffi::OsString;
use vessel_proc::{CommandDiagnostics, CommandSpec};
use vessel_proto::terminal_profiles::{ProfileError, ProfileSelection, ProfileSettings, Prompt};
use vessel_proto::terminals::TerminalKind;
use vessel_workspace::paths::Root;

#[derive(Debug)]
pub(super) struct PreparedTerminal {
    spec: CommandSpec,
    kind: TerminalKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum TerminalPreparationError {
    #[error("terminal launch profile is no longer available")]
    Missing,
    #[error("the terminal launch profile accepts no initial prompt")]
    ReadOnly,
    #[error("terminal launch profile preparation failed: {0}")]
    Profile(#[from] ProfileError),
}

impl PreparedTerminal {
    pub(super) fn profile(
        settings: &ProfileSettings,
        selection: &ProfileSelection,
        prompt: &Prompt,
        root: &Root,
        removed: &[OsString],
    ) -> Result<Self, TerminalPreparationError> {
        if selection.revision != settings.revision {
            return Err(ProfileError::StaleRevision.into());
        }
        let profile = settings
            .profiles
            .get(&selection.profile)
            .ok_or(TerminalPreparationError::Missing)?;
        if !profile.takes_prompt() && !prompt.as_str().is_empty() {
            return Err(TerminalPreparationError::ReadOnly);
        }
        let resolved = profile.resolve(prompt)?;
        let shell = Shell {
            program: resolved.command().as_str().into(),
            args: resolved
                .args()
                .iter()
                .map(|arg| arg.as_str().to_owned())
                .collect(),
        };
        let spec =
            shell_spec(&shell, root.path(), removed).with_diagnostics(CommandDiagnostics::Redacted);
        Ok(Self {
            spec,
            kind: TerminalKind::Profile {
                profile: selection.profile.clone(),
                revision: selection.revision,
            },
        })
    }

    pub(super) fn blank(shell: &Shell, root: &Root, removed: &[OsString]) -> Self {
        Self {
            spec: shell_spec(shell, root.path(), removed)
                .with_diagnostics(CommandDiagnostics::Redacted),
            kind: TerminalKind::Shell {
                program: shell.program.clone(),
            },
        }
    }

    /// The later receipted actor owns these actual values; metadata contains no
    /// expanded command for profile mode. No first prompt is sent here.
    pub(super) fn into_parts(self) -> (CommandSpec, TerminalKind) {
        (self.spec, self.kind)
    }
}

#[cfg(test)]
mod tests;
