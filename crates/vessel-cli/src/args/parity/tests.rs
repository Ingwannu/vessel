//! Argument assertions cover ambiguous positional and global-option combinations.
use super::{AgentCommand, ParityCommand, Presentation};
use crate::args::{Cli, Command};
use clap::{CommandFactory as _, Parser as _};

#[test]
fn complete_command_graph_has_no_duplicate_flags_or_invalid_groups() {
    Cli::command().debug_assert();
}
#[test]
fn prompt_sources_keep_the_final_agent_id_unambiguous() {
    for args in [
        vec!["vessel", "agent", "send", "hello", "ab12"],
        vec!["vessel", "agent", "send", "ab12", "--prompt", "hello"],
        vec![
            "vessel",
            "agent",
            "send",
            "ab12",
            "--prompt-file",
            "input.txt",
        ],
    ] {
        let cli = Cli::try_parse_from(args).unwrap();
        assert!(matches!(
            cli.command,
            Command::Parity(ParityCommand::Agent {
                command: AgentCommand::Send(_)
            })
        ));
    }
    for args in [
        vec![
            "vessel", "agent", "send", "hello", "ab12", "--prompt", "other",
        ],
        vec!["vessel", "agent", "send", "ab12"],
        vec!["vessel", "stop"],
        vec!["vessel", "stop", "ab12", "--all"],
    ] {
        assert!(Cli::try_parse_from(args).is_err());
    }
}
#[test]
fn global_precedence_and_connection_alias_conflicts_are_explicit() {
    let cli =
        Cli::try_parse_from(["vessel", "--format", "yaml", "ls", "--json", "--quiet"]).unwrap();
    assert_eq!(cli.output.presentation(), Presentation::Quiet);
    let cli = Cli::try_parse_from(["vessel", "--format", "yaml", "ls", "--json"]).unwrap();
    assert_eq!(cli.output.presentation(), Presentation::Json);
    assert!(
        Cli::try_parse_from(["vessel", "--host", "localhost", "--home", "/tmp/x", "ls"]).is_err()
    );
    // Legacy tests and SSH nodes need a separate local data directory.
    assert!(
        Cli::try_parse_from([
            "vessel",
            "--node",
            "localhost",
            "--data-dir",
            "/tmp/x",
            "ls"
        ])
        .is_ok()
    );
}
