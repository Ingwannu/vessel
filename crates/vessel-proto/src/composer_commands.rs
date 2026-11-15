//! Command catalog contract for the composer; discovery never sends a prompt.
//! The pending method remains unavailable until the integrator assigns its feature.

use serde::{Deserialize, Serialize};

use crate::{
    AgentCommand, Conversation, HarnessRef, ModelId, OptionChoice, ProfileName, ProtocolVersion,
};

/// Assigned only after the native catalog and node discovery consumers are ready.
/// Until assignment, kind metadata is removed from every peer-visible command list.
pub const COMPOSER_COMMANDS: Option<ProtocolVersion> = None;

/// A provider's explicit distinction decides whether a command is useful inline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CommandKind {
    /// A root-only provider/session operation.
    Command,
    /// A skill may be referenced inside a prompt.
    Skill,
    /// Future metadata is readable but cannot enable inline completion.
    #[serde(other)]
    Unknown,
}

/// Exact draft launch configuration; every field participates in catalog identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DraftCommandConfig {
    /// The selected installed provider or adapter.
    pub harness: HarnessRef,
    /// A selected account/profile, absent for the node's default.
    pub profile: Option<ProfileName>,
    /// The registered project/worktree, resolved through the workspace authority.
    pub root: crate::workspace::WorkspaceRoot,
    /// A validated subdirectory within that authorized root, or the root itself.
    pub directory: Option<crate::workspace::RelPath>,
    /// The selected provider model.
    pub model: Option<ModelId>,
    /// Mode, thinking and other validated provider choices.
    pub options: Vec<OptionChoice>,
}

/// Prompt-free draft query, registered as a pending method until integration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetDraftCommands {
    /// The complete draft configuration prevents reusing another draft's skills.
    pub config: DraftCommandConfig,
}

/// Command order remains the provider's; unsupported discovery is an error instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CommandCatalog {
    /// Commands and skills offered by this exact target.
    pub commands: Vec<AgentCommand>,
}

pub(crate) fn visible(assigned: Option<ProtocolVersion>, peer: ProtocolVersion) -> bool {
    assigned.is_some_and(|since| peer >= since)
}

pub(crate) fn commands_for(
    commands: &[AgentCommand],
    protocol: ProtocolVersion,
    assigned: Option<ProtocolVersion>,
) -> Option<Vec<AgentCommand>> {
    (!visible(assigned, protocol)
        && commands
            .iter()
            .any(|command| command.kind.is_some() || command.native_command.is_some()))
    .then(|| {
        commands
            .iter()
            .map(|command| AgentCommand {
                kind: None,
                native_command: None,
                ..command.clone()
            })
            .collect()
    })
}

impl crate::SessionEvent {
    pub(crate) fn composer_commands_for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        let Self::CommandsChanged { commands } = self else {
            return None;
        };
        let commands = commands_for(commands, protocol, COMPOSER_COMMANDS)?;
        let projected = Self::CommandsChanged { commands };
        projected.for_protocol(protocol).or(Some(projected))
    }
}

impl Conversation {
    pub(crate) fn composer_for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        let commands = commands_for(&self.commands, protocol, COMPOSER_COMMANDS);
        let usage = self
            .usage
            .as_ref()
            .and_then(|usage| usage.for_protocol(protocol));
        if commands.is_none() && usage.is_none() {
            return None;
        }
        let mut conversation = self.clone();
        if let Some(commands) = commands {
            conversation.commands = commands;
        }
        if let Some(usage) = usage {
            conversation.usage = Some(usage);
        }
        Some(conversation)
    }
}

#[cfg(test)]
mod tests;
