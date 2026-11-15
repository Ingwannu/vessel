//! Public help and parse contracts; no node, home, shell or service is started.
use super::*;
use clap::CommandFactory as _;

#[test]
fn help_keeps_the_run_options_and_every_subcommand_description() {
    Args::command().debug_assert();
    let mut command = Args::command()
        .term_width(0)
        .mut_args(|argument| argument.hide_env_values(true));
    let help = command.render_long_help().to_string();
    for sentence in [
        "Vessel node daemon",
        "Usage: vesseld [OPTIONS]",
        "Bridge stdin and stdout to this machine's node, starting it if none runs.",
        "Print this binary's version and target triple",
        "Delete installed vesseld binaries that are no longer needed",
        "Run a relay that nodes register with and clients reach them through.",
        "--lsp-config <LSP_CONFIG>",
        "--lsp-root <LSP_ROOTS>",
        "--lsp-full-analysis-root <LSP_FULL_ROOTS>",
        "--agent-handshake-timeout <SECONDS>",
        "--park-after <MINUTES>",
        "--relay-token-file <PATH>",
        "--client-address-header <CLIENT_ADDRESS_HEADER>",
        "--trusted-proxy <TRUSTED_PROXIES>",
    ] {
        assert!(help.contains(sentence), "missing public help: {sentence}");
    }
    assert!(
        !help.contains("--detached"),
        "internal readiness is not a public option"
    );
    let relay = command.find_subcommand_mut("relay").unwrap().clone();
    let help = relay
        .term_width(0)
        .mut_args(|argument| argument.hide_env_values(true))
        .render_long_help()
        .to_string();
    for flag in [
        "--max-nodes",
        "--max-clients",
        "--reserved-registering",
        "--max-node-clients-per-address",
    ] {
        assert!(help.contains(flag), "missing relay help: {flag}");
    }
}

#[test]
fn parsing_keeps_entrypoints_defaults_ranges_and_conflicting_modes() {
    let args = Args::try_parse_from([
        "vesseld",
        "--bind",
        "none",
        "--name",
        "isolated",
        "--park-after",
        "0",
    ])
    .unwrap();
    assert!(args.command.is_none());
    assert!(matches!(args.run.bind, Bind::None));
    assert_eq!(args.run.name.as_deref(), Some("isolated"));
    assert_eq!(args.run.park_after, Some(0));
    for (argv, expected) in [
        (vec!["vesseld", "attach"], "attach"),
        (vec!["vesseld", "version"], "version"),
        (vec!["vesseld", "cleanup", "--keep", "3"], "cleanup"),
    ] {
        let command = Args::try_parse_from(argv).unwrap().command.unwrap();
        let actual = match command {
            Command::Attach => "attach",
            Command::Version => "version",
            Command::Cleanup { keep } => {
                assert_eq!(keep, 3);
                "cleanup"
            }
            Command::Relay(_) => "relay",
        };
        assert_eq!(actual, expected);
    }
    let args = Args::try_parse_from([
        "vesseld",
        "relay",
        "--token-file",
        "/synthetic/token",
        "--max-nodes",
        "8",
    ])
    .unwrap();
    let Some(Command::Relay(relay)) = args.command else {
        panic!("relay command missing")
    };
    assert_eq!(relay.max_nodes, 8);
    for argv in [
        vec!["vesseld", "--bind", "none", "version"],
        vec!["vesseld", "--park-after", "10081"],
        vec!["vesseld", "--agent-handshake-timeout", "0"],
        vec![
            "vesseld",
            "relay",
            "--token-file",
            "/synthetic/token",
            "--max-nodes",
            "0",
        ],
    ] {
        assert!(Args::try_parse_from(argv).is_err());
    }
}

#[test]
fn both_tool_suppression_spellings_are_process_start_options_only() {
    for flag in ["--no-mcp", "--no-inject-mcp"] {
        let args = Args::try_parse_from(["vesseld", flag]).unwrap();
        assert!(args.run.no_mcp);
        assert!(args.command.is_none());
        assert!(Args::try_parse_from(["vesseld", flag, "version"]).is_err());
    }
    assert!(!Args::try_parse_from(["vesseld"]).unwrap().run.no_mcp);
}
