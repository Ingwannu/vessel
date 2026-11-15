//! Typed CLI orchestration over existing calls; no method registration belongs here.

mod agents;
pub(crate) mod failure;
mod heartbeat;
mod hooks;
mod hosts;
mod observe;
pub(crate) mod output;
pub(crate) mod projects;
pub(crate) mod prompt_input;
pub(crate) mod schedule_output;
#[cfg(test)]
mod structured_output;
mod terminals;
mod workspaces;

use crate::{
    args::{
        Target,
        parity::{AgentCommand, ParityCommand, Presentation},
    },
    error::CliError,
};

/// Route root shortcuts to exactly the same handler as their agent spelling.
pub(crate) async fn run(
    target: &Target,
    output: Presentation,
    command: ParityCommand,
) -> Result<(), CliError> {
    let command = match command {
        ParityCommand::Hooks(invocation) => return hooks::run(invocation),
        ParityCommand::Provider { command } => return Err(command.pending()),
        ParityCommand::Clone(args) => return projects::clone(target, output, args).await,
        ParityCommand::Workspace { command } => {
            return workspaces::run(target, output, command).await;
        }
        ParityCommand::Heartbeat { command } => {
            return heartbeat::run(target, output, command).await;
        }
        ParityCommand::Terminal { command } => {
            return terminals::run(target, output, command).await;
        }
        ParityCommand::Host { command } => return hosts::run(target, output, command).await,
        ParityCommand::Agent { command } => command,
        ParityCommand::Ls(args) => AgentCommand::Ls(args),
        ParityCommand::Inspect { id } => AgentCommand::Inspect { id },
        ParityCommand::Wait(args) => AgentCommand::Wait(args),
        ParityCommand::Stop(args) => AgentCommand::Stop(args),
        ParityCommand::Delete(args) => AgentCommand::Delete(args),
        ParityCommand::Archive { id, force } => AgentCommand::Archive { id, force },
    };
    agents::run(target, output, command).await
}
