//! Optional command classification values; this carrier registers no RPC or producer.
use crate::{AgentCommand, Conversation, ProtocolVersion};
use serde::{Deserialize, Serialize};

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

pub(crate) fn visible(assigned: Option<ProtocolVersion>, peer: ProtocolVersion) -> bool {
    assigned.is_some_and(|since| peer >= since)
}

pub(crate) fn commands_for(
    commands: &[AgentCommand],
    protocol: ProtocolVersion,
    assigned: Option<ProtocolVersion>,
) -> Option<Vec<AgentCommand>> {
    (!visible(assigned, protocol) && commands.iter().any(|command| command.kind.is_some())).then(
        || {
            commands
                .iter()
                .map(|command| AgentCommand {
                    kind: None,
                    ..command.clone()
                })
                .collect()
        },
    )
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
