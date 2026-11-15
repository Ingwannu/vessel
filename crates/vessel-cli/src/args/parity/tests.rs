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
    let mut conflicting =
        Cli::try_parse_from(["vessel", "--host", "localhost", "--home", "/tmp/x", "ls"]).unwrap();
    assert!(
        conflicting
            .connection
            .apply(&mut conflicting.target, &conflicting.command)
            .is_err()
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

#[test]
fn pending_consumers_parse_complete_forms_and_keep_scope_conflicts_explicit() {
    for args in [
        vec![
            "vessel",
            "run",
            "prompt",
            "--provider",
            "codex/team/model",
            "--new-workspace",
            "worktree",
            "--worktree-mode",
            "checkout-pr",
            "--pr-number",
            "12",
            "--forge",
            "github",
            "-d",
        ],
        vec![
            "vessel",
            "import",
            "provider-conversation",
            "--provider",
            "codex",
        ],
        vec!["vessel", "usage", "--agent", "ab12", "--refresh", "--json"],
        vec!["vessel", "usage", "ls", "--agent", "ab12", "--refresh"],
        vec!["vessel", "usage", "inspect", "quota-id", "--refresh"],
        vec![
            "vessel",
            "provider",
            "models",
            "future-provider",
            "--thinking",
        ],
        vec![
            "vessel",
            "worktree",
            "create",
            "--mode",
            "checkout-pr",
            "--pr-number",
            "1",
            "--cwd",
            "/repo",
        ],
        vec!["vessel", "worktree", "archive", "branch", "--cwd", "/repo"],
        vec![
            "vessel",
            "schedule",
            "create",
            "prompt",
            "--provider",
            "codex/model",
            "--cwd",
            "/repo",
            "--every",
            "15m",
            "--run-now",
            "--expires-in",
            "1h",
        ],
        vec![
            "vessel",
            "schedule",
            "update",
            "name",
            "--name",
            "",
            "--no-max-runs",
            "--no-expires-in",
        ],
    ] {
        let args = args.into_iter().map(std::ffi::OsString::from).collect();
        assert!(Cli::try_parse_from(super::normalize(args)).is_ok());
    }
}

#[test]
fn pending_consumer_scopes_and_limit_resets_conflict_explicitly() {
    for args in [
        vec!["vessel", "usage", "--agent", "ab12", "inspect", "quota"],
        vec!["vessel", "usage", "inspect", "quota", "--agent", "ab12"],
        vec![
            "vessel",
            "schedule",
            "update",
            "name",
            "--no-max-runs",
            "--max-runs",
            "1",
        ],
        vec![
            "vessel",
            "agent",
            "run",
            "prompt",
            "--provider",
            "codex",
            "--workspace",
            "id",
            "--new-workspace",
            "local",
        ],
    ] {
        assert!(Cli::try_parse_from(args).is_err());
    }
}

#[test]
fn native_init_scan_home_never_replaces_the_node_data_directory() {
    for args in [
        vec![
            "vessel",
            "--data-dir",
            "/tmp/node-data",
            "init",
            "--home",
            "/tmp/user-scan",
        ],
        vec![
            "vessel",
            "--node",
            "localhost",
            "--data-dir",
            "/tmp/node-data",
            "init",
            "--home",
            "/tmp/user-scan",
        ],
        vec![
            "vessel",
            "--host",
            "localhost",
            "--data-dir",
            "/tmp/node-data",
            "init",
            "--home",
            "/tmp/user-scan",
        ],
    ] {
        let mut cli = Cli::try_parse_from(args).unwrap();
        assert!(
            matches!(&cli.command, Command::Init(args) if args.home.as_deref() == Some(std::path::Path::new("/tmp/user-scan")))
        );
        cli.connection.apply(&mut cli.target, &cli.command).unwrap();
        assert_eq!(
            cli.target.data_dir.as_deref(),
            Some(std::path::Path::new("/tmp/node-data"))
        );
    }
    let mut cli = Cli::try_parse_from(["vessel", "ls", "--home", "/tmp/node-data"]).unwrap();
    cli.connection.apply(&mut cli.target, &cli.command).unwrap();
    assert_eq!(
        cli.target.data_dir.as_deref(),
        Some(std::path::Path::new("/tmp/node-data"))
    );
    let mut cli = Cli::try_parse_from([
        "vessel",
        "--data-dir",
        "/tmp/one",
        "ls",
        "--home",
        "/tmp/two",
    ])
    .unwrap();
    assert!(cli.connection.apply(&mut cli.target, &cli.command).is_err());
}

#[test]
fn images_do_not_replace_the_required_send_prompt_source() {
    assert!(
        Cli::try_parse_from([
            "vessel",
            "agent",
            "send",
            "ab12",
            "--image",
            "not-opened.png"
        ])
        .is_err()
    );
    assert!(
        Cli::try_parse_from([
            "vessel",
            "agent",
            "send",
            "ab12",
            "--prompt",
            "describe",
            "--image",
            "not-opened.png"
        ])
        .is_ok()
    );
}
