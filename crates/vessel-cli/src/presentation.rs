//! Admit explicit presentation only when a handler consumes it, before connecting.
//! Raw attach/log streams keep their contract by refusing data-format selection.

use crate::{
    args::{
        Cli, Command, ConversationCommand, ProjectCommand, ScheduleCommand, SessionCommand,
        UsageCommand,
        parity::{AgentCommand, ParityCommand, Presentation},
    },
    error::CliError,
};

pub(crate) fn check(cli: &Cli) -> Result<(), CliError> {
    let Some(requested) = cli.output.requested() else {
        return Ok(());
    };
    if consumes(&cli.command) {
        return Ok(());
    }
    // Compare already owns its JSON switch. It must continue to work without
    // enabling other output formats that its handler does not consume.
    if let Command::Session {
        command: SessionCommand::Conversation(ConversationCommand::Compare(args)),
    } = &cli.command
        && args.json
        && requested == Presentation::Json
    {
        return Ok(());
    }
    Err(CliError::UnsupportedPresentation)
}

/// Only admitted paths consume output or return a typed preflight refusal.
fn consumes(command: &Command) -> bool {
    if matches!(command, Command::Models) {
        return true;
    }
    if let Command::Parity(command) = command {
        return !matches!(
            command,
            ParityCommand::Hooks(_)
                | ParityCommand::Agent {
                    command: AgentCommand::Attach { .. }
                }
        );
    }
    if let Command::Project { command } = command {
        return matches!(
            command,
            ProjectCommand::List
                | ProjectCommand::Create { .. }
                | ProjectCommand::Rename { .. }
                | ProjectCommand::Delete { .. }
        );
    }
    if let Command::Schedule { command } = command {
        return !matches!(command, ScheduleCommand::Preview { .. });
    }
    if let Command::Usage { command, .. } = command {
        return command.as_ref().is_none_or(|command| {
            matches!(command, UsageCommand::Ls(_) | UsageCommand::Inspect { .. })
        });
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, Parser};
    #[test]
    fn shared_output_root_graph_and_command_admission() {
        Cli::command().debug_assert();
        for args in [
            vec!["vessel", "status"],
            vec!["vessel", "sessions"],
            vec!["vessel", "status", "--no-color"],
            vec!["vessel", "models", "--json"],
            vec!["vessel", "models", "--format", " YAML "],
            vec!["vessel", "models", "--quiet"],
            vec!["vessel", "models", "--no-headers"],
            vec!["vessel", "ls", "--json"],
            vec!["vessel", "project", "ls", "--json"],
            vec!["vessel", "schedule", "ls", "--json"],
        ] {
            let cli = Cli::try_parse_from(args).unwrap();
            assert!(check(&cli).is_ok());
        }
        for args in [
            vec!["vessel", "sessions", "--json"],
            vec!["vessel", "status", "--quiet"],
            vec!["vessel", "status", "-o", "table"],
            vec!["vessel", "log", "abcd", "--json"],
            vec!["vessel", "agent", "attach", "abcd", "--json"],
            vec!["vessel", "project", "commands", "abcd", "--json"],
        ] {
            let cli = Cli::try_parse_from(args).unwrap();
            assert!(matches!(
                check(&cli),
                Err(CliError::UnsupportedPresentation)
            ));
        }
        let compare = [
            "vessel",
            "session",
            "compare",
            "--project",
            "00000000-0000-0000-0000-000000000001",
            "--base",
            "main",
            "--targets",
            "unused.json",
            "--json",
        ];
        let cli = Cli::try_parse_from(compare).unwrap();
        assert!(
            matches!(cli.command, Command::Session { command: SessionCommand::Conversation(ConversationCommand::Compare(ref args)) } if args.json)
        );
        assert!(check(&cli).is_ok());
    }
    #[tokio::test]
    async fn shared_output_refusal_precedes_even_connection_parsing() {
        let cli = Cli::try_parse_from(["vessel", "--node", "ws://[", "status", "--json"]).unwrap();
        assert!(matches!(
            crate::commands::run(cli).await,
            Err(CliError::UnsupportedPresentation)
        ));
    }
}
