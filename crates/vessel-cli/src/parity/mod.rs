//! Typed CLI orchestration over existing calls; no method registration belongs here.

mod agents;
mod heartbeat;
mod hosts;
mod observe;
pub(crate) mod output;
mod terminals;

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
