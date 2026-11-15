//! Paseo-compatible invocation shapes. Node contracts and effects live elsewhere.

mod agent;
mod clone;
mod connection;
mod duration;
mod heartbeat;
mod hooks;
mod host;
mod identifier;
mod invocation;
pub(crate) mod output;
mod provider;
mod provider_argument;
mod schedule;
mod terminal;
mod workspace;
mod worktree;

pub(crate) use agent::{AgentCommand, AgentList, AgentRun, AgentSend, Selection, Wait};
pub(crate) use clone::{Clone, CloneProtocol};
pub(crate) use connection::ConnectionOptions;
pub(crate) use duration::WaitBudget;
pub(crate) use heartbeat::{HeartbeatCommand, ScheduleSelector};
pub(crate) use hooks::HookInvocation;
pub(crate) use host::{HostCommand, destination as host_destination, metadata as host_metadata};
pub(crate) use invocation::normalize;
pub(crate) use output::{OutputOptions, Presentation};
pub(crate) use schedule::ScheduleParityArgs;
pub(crate) use terminal::{TerminalCommand, TerminalSelector};
pub(crate) use workspace::WorkspaceCommand;
pub(crate) use worktree::WorktreeCreate;

use crate::selector::SessionSelector;
use clap::Subcommand;

/// Root shortcuts share the same typed handlers as the agent group.
#[derive(Debug, Subcommand)]
pub(crate) enum ParityCommand {
    /// Record authenticated terminal hook activity when its node contract is available.
    Hooks(HookInvocation),
    /// Provider snapshots/models/diagnostics await the canonical provider contract.
    Provider {
        #[command(subcommand)]
        command: provider::ProviderCommand,
    },
    /// Clone a complete Git remote or explicitly selected GitHub shorthand.
    Clone(Clone),
    /// Manage real workspace identities independently of projects/worktrees.
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
    /// Schedule recurring prompts only for this calling agent.
    Heartbeat {
        #[command(subcommand)]
        command: HeartbeatCommand,
    },
    /// List, create, inspect or send keys to workspace-owned terminals.
    Terminal {
        #[command(subcommand)]
        command: TerminalCommand,
    },
    /// Registrations reference the node's existing trusted fleet/paired destinations.
    Host {
        #[command(subcommand)]
        command: HostCommand,
    },
    /// Manage agent conversations (provider discovery remains `agents`).
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
    /// List agents in the current directory, or across the node with --global.
    Ls(AgentList),
    /// Inspect an agent by its full ID or unique prefix.
    Inspect { id: SessionSelector },
    /// Wait for an agent turn, including permission and timeout results.
    Wait(Wait),
    /// Interrupt active turns without terminating idle agent processes.
    Stop(Selection),
    /// Delete agents after interrupting any active turn.
    Delete(Selection),
    /// Archive an agent, protecting an active turn unless --force is supplied.
    Archive {
        id: SessionSelector,
        #[arg(long)]
        force: bool,
    },
}

#[cfg(test)]
mod tests;
