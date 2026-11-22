//! ADR 0117 over the wire, at protocol 45: the node keeps a list of terminal
//! profiles and launches one by id; an older peer is refused both fields and
//! never shown the list. The profile runs the testkit's `fake-tui` as
//! `claude`, never a real agent.

use std::path::PathBuf;

use vessel_client::NodeClient;
use vessel_node::{Node, Shell};
use vessel_proto::settings::{GetNodeSettings, SetNodeSettings};
use vessel_proto::terminal_profiles::{
    Argument, Id, Name, Profile, ProfileLaunch, Profiles, Program, TERMINAL_PROFILES_PROTOCOL,
};
use vessel_proto::terminals::{CreateTerminal, TerminalPlace};
use vessel_proto::{
    Call, ErrorCode, EventSeq, Outcome, ProtocolVersion, Reply, Request as _, SessionEvent,
};
use vessel_testkit::Fake;

use crate::support::{Fixture, step, within};
use crate::workspace_terminals::as_older;

/// A node whose search path holds only `claude`, the fake TUI.
async fn node_with_claude(fixture: &Fixture, bin: PathBuf) -> (Node, NodeClient) {
    std::fs::create_dir_all(&bin).unwrap();
    std::os::unix::fs::symlink(Fake::Tui.path().unwrap(), bin.join("claude")).unwrap();
    let node = fixture
        .start_with(|config| {
            config.terminals.shell = Some(Shell::plain("/bin/sh"));
            config.terminals.search_path = Some(bin.into_os_string());
        })
        .await;
    let owner = fixture.connect(&node).await;
    (node, owner)
}

fn kept() -> Profiles {
    Profiles::new(vec![
        Profile::new(
            Id::parse("profile_m1_a").unwrap(),
            Name::parse("Fake Claude").unwrap(),
            Program::parse("claude").unwrap(),
            vec![Argument::parse("--fake").unwrap()],
            None,
        )
        .unwrap(),
    ])
    .unwrap()
}

fn launch(path: PathBuf) -> CreateTerminal {
    CreateTerminal {
        place: TerminalPlace::Folder { path },
        size: None,
        title: None,
        profile: Some(ProfileLaunch {
            id: Id::parse("profile_m1_a").unwrap(),
            prompt: None,
        }),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_kept_profile_is_shown_and_launched_over_the_wire() {
    within(async {
        assert_eq!(TERMINAL_PROFILES_PROTOCOL, Some(ProtocolVersion::new(45)));
        let fixture = Fixture::new();
        let (node, owner) = node_with_claude(&fixture, fixture.cwd().join("bin")).await;
        assert_eq!(
            owner
                .call(GetNodeSettings::default())
                .await
                .unwrap()
                .terminal_profiles,
            None
        );
        let set = owner
            .call(SetNodeSettings {
                terminal_profiles: Some(kept()),
                ..SetNodeSettings::default()
            })
            .await
            .unwrap();
        assert_eq!(set.terminal_profiles, Some(kept()));
        let read = owner.call(GetNodeSettings::default()).await.unwrap();
        assert_eq!(read.terminal_profiles, Some(kept()));
        let terminal = owner.call(launch(fixture.cwd())).await.unwrap();
        assert_eq!(
            terminal.title.as_ref().map(ToString::to_string).as_deref(),
            Some("Fake Claude")
        );
        let mut stream = owner
            .follow_terminal(terminal.id, EventSeq::ZERO)
            .await
            .unwrap();
        let mut seen = String::new();
        step("the profile's program prints ready", async {
            while !seen.contains("ready") {
                let Some(record) = stream.next().await else {
                    panic!("it ended before ready: {seen:?}");
                };
                if let SessionEvent::Terminal(output) = record.unwrap().event {
                    seen.push_str(&String::from_utf8_lossy(output.data.bytes()));
                }
            }
        })
        .await;
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_older_peer_is_refused_the_profiles_and_never_shown_them() {
    within(async {
        let fixture = Fixture::new();
        let (node, owner) = node_with_claude(&fixture, fixture.cwd().join("bin")).await;
        let _set = owner
            .call(SetNodeSettings {
                terminal_profiles: Some(kept()),
                ..SetNodeSettings::default()
            })
            .await
            .unwrap();
        let since = TERMINAL_PROFILES_PROTOCOL.unwrap();
        let older = ProtocolVersion::new(since.get().saturating_sub(1));
        let outcomes = as_older(
            &fixture,
            &node,
            older,
            vec![
                GetNodeSettings::default().into_call(),
                SetNodeSettings {
                    terminal_profiles: Some(kept()),
                    ..SetNodeSettings::default()
                }
                .into_call(),
                Call::CreateTerminal(launch(fixture.cwd())),
            ],
        )
        .await;
        let [read, set, create] = outcomes.as_slice() else {
            panic!("three answers: {outcomes:?}");
        };
        let Outcome::Ok(Reply::GetNodeSettings(settings)) = read else {
            panic!("the settings: {read:?}");
        };
        assert_eq!(settings.terminal_profiles, None);
        for refused in [set, create] {
            assert!(
                matches!(refused, Outcome::Err(error) if error.code == ErrorCode::Unsupported),
                "{refused:?}"
            );
        }
        node.stop().await.unwrap();
    })
    .await;
}
