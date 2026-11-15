//! The set last approved (protocol 11, the join of wave 2's first half):
//! kept with each approval of a set the node described, shown beside a set
//! that is not approved, the same after a restart, and gone with its
//! withdrawal.

use vessel_proto::workspace::{ApprovedCommands, CommandsDigest, ProjectCommand, ProjectCommands};

use super::Fixture;

const FIRST: &str = "[worktree]\nsetup = [\"echo one\"]\n";
const CHANGED: &str = "[worktree]\nsetup = [\"echo two\"]\nteardown = [\"echo bye\"]\n";

/// The digest, whether it is approved, and the set last approved.
fn parts(commands: ProjectCommands) -> (CommandsDigest, bool, Option<ApprovedCommands>) {
    match commands {
        ProjectCommands::Listed {
            digest,
            approved,
            last_approved,
            ..
        } => (digest, approved, last_approved.map(|last| *last)),
        other @ (ProjectCommands::None | ProjectCommands::Invalid { .. }) => {
            panic!("no set: {other:?}")
        }
    }
}

#[tokio::test]
async fn a_changed_set_shows_the_one_last_approved() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", Some(FIRST)).await;
    let workspaces = &fixture.workspaces;
    let (first, approved, last) = parts(workspaces.commands(project.id, None).await.unwrap());
    assert!(!approved && last.is_none(), "nothing was approved yet");
    let (_, approved, last) = parts(
        workspaces
            .approve(project.id, first.clone(), true)
            .await
            .unwrap(),
    );
    assert!(approved && last.is_none(), "an approved set needs no other");

    std::fs::write(project.root.join("vessel.toml"), CHANGED).unwrap();
    let (changed, approved, last) = parts(workspaces.commands(project.id, None).await.unwrap());
    assert!(!approved);
    let last = last.expect("the set last approved");
    assert_eq!(last.digest, first);
    let setup: Vec<&str> = last
        .commands
        .setup
        .iter()
        .map(ProjectCommand::as_str)
        .collect();
    assert_eq!(setup, ["echo one"]);
    assert!(last.commands.teardown.is_empty());

    // A digest the node never described is approved, but covers nothing
    // the node can show: the last one it can show stays.
    let unseen: CommandsDigest = "b".repeat(64).parse().unwrap();
    let (_, _, still) = parts(workspaces.approve(project.id, unseen, true).await.unwrap());
    assert_eq!(still.map(|set| set.digest), Some(first.clone()));

    // The store keeps it.
    let restarted = fixture.restart().await;
    let (again, approved, kept) = parts(restarted.commands(project.id, None).await.unwrap());
    assert_eq!(again, changed);
    assert!(!approved);
    assert_eq!(kept, Some(last));

    // Withdrawn, it is not "approved last" any more.
    let (_, _, none) = parts(restarted.approve(project.id, first, false).await.unwrap());
    assert_eq!(none, None);
    // Approving the changed set: approved, nothing beside it.
    let (_, approved, none) = parts(restarted.approve(project.id, changed, true).await.unwrap());
    assert!(approved && none.is_none());
}

#[tokio::test]
async fn terminal_approval_survives_restart_and_a_changed_command_asks_again() {
    let fixture = Fixture::new().await;
    let text = "[[worktree.terminals]]\nname = 'logs'\ncommand = 'echo first'\n";
    let project = fixture.project("terminal-approval", Some(text)).await;
    let (digest, approved, _) = parts(fixture.workspaces.commands(project.id, None).await.unwrap());
    assert!(!approved);
    let _approved = fixture
        .workspaces
        .approve(project.id, digest.clone(), true)
        .await
        .unwrap();
    let changed = text.replace("echo first", "echo second");
    std::fs::write(project.root.join("vessel.toml"), changed).unwrap();
    let restarted = fixture.restart().await;
    let current = restarted.commands(project.id, None).await.unwrap();
    let ProjectCommands::Listed {
        approved,
        terminals,
        last_approved,
        digest: next,
        ..
    } = current
    else {
        panic!("terminal-only file was not described");
    };
    assert!(!approved);
    assert_ne!(next, digest);
    assert_eq!(terminals[0].command.as_str(), "echo second");
    let last = last_approved.unwrap();
    assert_eq!(last.digest, digest);
    assert_eq!(last.commands.terminals.len(), 1);
    assert_eq!(
        last.commands.terminals[0].name.as_ref().unwrap().as_str(),
        "logs"
    );
    assert_eq!(last.commands.terminals[0].command.as_str(), "echo first");
}

#[tokio::test]
async fn allocation_only_approvals_survive_restart_and_removal_keeps_the_prior_block() {
    let fixture = Fixture::new().await;
    let config =
        "[worktree.service_ports]\nrange = '7000-7009'\nport_script = 'tools/port-approval'\n";
    let project = fixture.project("allocation-approval", Some(config)).await;
    let initial = fixture.workspaces.commands(project.id, None).await.unwrap();
    let ProjectCommands::Listed {
        digest,
        service_ports: Some(ports),
        ..
    } = initial
    else {
        panic!("allocation-only file must remain a described set");
    };
    assert_eq!(ports.range().unwrap().to_string(), "7000-7009");
    assert_eq!(ports.port_script().unwrap().as_str(), "tools/port-approval");
    let _approved = fixture
        .workspaces
        .approve(project.id, digest.clone(), true)
        .await
        .unwrap();
    std::fs::write(project.root.join("vessel.toml"), FIRST).unwrap();
    let restarted = fixture.restart().await;
    let current = restarted.commands(project.id, None).await.unwrap();
    let ProjectCommands::Listed {
        service_ports,
        approved,
        last_approved,
        digest: changed,
        ..
    } = &current
    else {
        panic!("changed commands must be described");
    };
    assert!(service_ports.is_none() && !approved);
    assert_ne!(*changed, digest);
    let previous = last_approved.as_ref().unwrap();
    assert_eq!(previous.digest, digest);
    assert_eq!(previous.commands.service_ports.as_ref(), Some(&ports));
    assert!(matches!(
        current.for_protocol(vessel_proto::PROTOCOL_VERSION),
        Some(ProjectCommands::Invalid { .. })
    ));
}
