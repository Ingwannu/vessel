use super::*;
use vessel_proto::workspace::FileTerminal;

fn commands(approved: bool) -> ProjectCommands {
    ProjectCommands::Listed {
        setup: vec![],
        teardown: vec![],
        scripts: vec![],
        service_ports: None,
        terminals: vec![FileTerminal {
            name: None,
            command: "echo terminal-only".parse().unwrap(),
        }],
        digest: "a".repeat(64).parse().unwrap(),
        approved,
        last_approved: None,
    }
}
#[test]
fn terminal_only_setup_requires_approval_before_the_empty_setup_branch() {
    assert!(plan(SetupChoice::Run, &commands(false)).is_none());
    assert!(matches!(
        plan(SetupChoice::Skip, &commands(false)),
        Some(Planned::Skipped)
    ));
    assert!(plan(SetupChoice::Run, &commands(true)).is_some());
}
