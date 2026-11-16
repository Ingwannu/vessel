//! Exercise the actual clap command edge before any node connector exists.
use super::*;
use clap::Parser;

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

fn parse(arguments: &[&str]) -> Command {
    Cli::try_parse_from(arguments).unwrap().command
}
const SESSION: &str = "00000000-0000-4000-8000-000000000001";

#[test]
fn allow_defaults_to_all_but_deny_requires_explicit_scope() {
    assert!(parse(&["permit", "allow", SESSION]).validate().is_ok());
    assert_eq!(
        parse(&["permit", "deny", SESSION]).validate(),
        Err(SelectionError)
    );
    assert!(
        parse(&["permit", "deny", SESSION, "--all"])
            .validate()
            .is_ok()
    );
    assert!(
        parse(&["permit", "deny", SESSION, "abcd"])
            .validate()
            .is_ok()
    );
    let Command::Allow { requests, .. } = parse(&["permit", "allow", SESSION]) else {
        panic!("allow")
    };
    assert_eq!(requests.scope(Direction::Allow), Ok(Scope::All));
    assert_eq!(requests.session, SESSION.parse().unwrap());
}

#[test]
fn explicit_all_takes_precedence_over_a_request_for_both_directions() {
    for name in ["allow", "deny"] {
        let command = parse(&["permit", name, SESSION, "abcd", "--all"]);
        let (requests, direction) = match command {
            Command::Allow { requests, .. } => (requests, Direction::Allow),
            Command::Deny { requests, .. } => (requests, Direction::Deny),
            Command::Ls => panic!("write"),
        };
        assert_eq!(requests.scope(direction), Ok(Scope::All));
    }
}

#[test]
fn invalid_input_is_refused_during_parse_and_valid_input_remains_opaque() {
    assert!(Cli::try_parse_from(["permit", "allow", SESSION, "--input", "{broken"]).is_err());
    let marker = r#"{"answers":{"Question 1":"private-test-text"}}"#;
    let command = parse(&["permit", "allow", SESSION, "--input", marker]);
    assert!(!format!("{command:?}").contains("private-test-text"));
    let Command::Allow {
        input: Some(input), ..
    } = command
    else {
        panic!("input")
    };
    assert_eq!(input.raw().as_text(), marker);
}

#[test]
fn denial_message_and_interrupt_are_distinct_and_preserved() {
    let command = parse(&[
        "permit",
        "deny",
        SESSION,
        "abcd",
        "--message",
        "synthetic-private-denial-message",
        "--interrupt",
    ]);
    assert!(!format!("{command:?}").contains("synthetic-private-denial-message"));
    assert_eq!(command.interruption(), Interruption::Interrupt);
    let Command::Deny { message, .. } = command else {
        panic!("deny")
    };
    assert_eq!(message.as_deref(), Some("synthetic-private-denial-message"));
    assert_eq!(
        parse(&["permit", "ls"]).interruption(),
        Interruption::KeepRunning
    );
}

#[test]
fn invalid_json_diagnostics_keep_the_source_without_echoing_the_argument() {
    let canary = "synthetic-private-invalid-json";
    let invalid = format!("{{\"answer\":\"{canary}\"");
    let error = Cli::try_parse_from(["permit", "allow", SESSION, "--input", &invalid]).unwrap_err();
    assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
    assert!(std::error::Error::source(&error).is_some());
    let mut cause: Option<&dyn std::error::Error> = Some(&error);
    while let Some(current) = cause {
        assert!(!current.to_string().contains(canary));
        assert!(!format!("{current:?}").contains(canary));
        cause = current.source();
    }
}

#[tokio::test]
async fn root_dispatch_refuses_staged_responses_before_even_parsing_a_node_address() {
    for name in ["allow", "deny"] {
        let cli = crate::args::Cli::try_parse_from([
            "vessel", "--node", "ws://[", "permit", name, SESSION, "--all",
        ])
        .unwrap();
        assert!(matches!(
            crate::commands::run(cli).await,
            Err(crate::error::CliError::Usage(_))
        ));
        let cli = crate::args::Cli::try_parse_from([
            "vessel", "--node", "ws://[", "permit", name, SESSION, "--all", "--json",
        ])
        .unwrap();
        assert!(matches!(
            crate::commands::run(cli).await,
            Err(crate::error::CliError::UnsupportedPresentation)
        ));
    }
}
