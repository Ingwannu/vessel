//! Moving a structured session to another profile end to end (milestone 4
//! Track H2): `session.move_profile` refuses what cannot move and changes
//! nothing then; a move continues the conversation under the new profile,
//! and the session keeps that profile across a restart; a moved session
//! holds its conversation under every profile it went through (ADR 0053's
//! one agent per conversation file). The homes are temporary directories
//! laid out as Claude Code would have written them; `fake-tui`, installed
//! as `claude`, answers the login check.

use std::path::Path;

use vessel_client::NodeClient;
use vessel_harness::fakes::claude_project;
use vessel_proto::{
    ErrorCode, EventSeq, HarnessKind, HarnessSessionId, ProfileName, SessionEvent, SessionId,
    SessionSpec, SessionStatus,
};
use vessel_rpc::CallError;

use crate::support::{Fixture, ask, read_until, status_of, structured_spec, terminal_spec, within};

fn name(text: &str) -> ProfileName {
    text.parse().unwrap()
}

/// Two logged-in Claude profiles, one that is not, and a Codex one, with
/// their homes under `homes`.
fn profiles_toml(homes: &Path) -> String {
    format!(
        r#"
[profiles.claude-alt]
harness = "claude"
env = {{ CLAUDE_CONFIG_DIR = '{alt}' }}

[profiles.claude-work]
harness = "claude"
env = {{ CLAUDE_CONFIG_DIR = '{work}' }}

[profiles.claude-out]
harness = "claude"
env = {{ CLAUDE_CONFIG_DIR = '{out}', FAKE_TUI_LOGGED_OUT = "1" }}

[profiles.codex-work]
harness = "codex"
"#,
        alt = homes.join("alt").display(),
        work = homes.join("work").display(),
        out = homes.join("out").display(),
    )
}

/// Three logged-in Claude profiles, `claude-<home>` for each home under
/// `homes`.
fn three_profiles(homes: &Path) -> String {
    use std::fmt::Write as _;
    let mut toml = String::new();
    for home in ["alt", "work", "third"] {
        let dir = homes.join(home);
        writeln!(
            toml,
            "[profiles.claude-{home}]\nharness = \"claude\"\nenv = {{ CLAUDE_CONFIG_DIR = '{}' }}",
            dir.display()
        )
        .unwrap();
    }
    toml
}

/// An import of `conversation` under `profile`: `session.create` resuming
/// it.
fn import(fixture: &Fixture, conversation: &HarnessSessionId, profile: &str) -> SessionSpec {
    let mut spec = structured_spec(fixture, Some(conversation.clone()));
    spec.profile = Some(name(profile));
    spec
}

/// `id` followed from its first record until its status is one `until`
/// accepts.
async fn until_status(client: &NodeClient, id: SessionId, until: impl Fn(&SessionStatus) -> bool) {
    let mut stream = client.follow(id, EventSeq::ZERO).await.unwrap();
    let mut seen = Vec::new();
    read_until(&mut stream, &mut seen, |record| {
        status_of(record).is_some_and(&until)
    })
    .await;
}

/// Refused because `holder` holds the conversation (`conflict`, naming
/// it), not for another reason.
fn taken<T: std::fmt::Debug>(result: Result<T, CallError>, holder: SessionId) {
    let Err(CallError::Remote(error)) = result else {
        panic!("expected a refusal, got {result:?}");
    };
    assert_eq!(error.code, ErrorCode::Conflict, "{error:?}");
    assert!(
        error.message.contains(&holder.to_string()),
        "{holder}: {error:?}"
    );
}

/// The node refused a move with `code`.
async fn refused(client: &NodeClient, session: SessionId, profile: &str, code: ErrorCode) {
    let Err(CallError::Remote(error)) = client
        .move_profile(session, Some(name(profile)), false)
        .await
    else {
        panic!("a move to {profile} must be refused");
    };
    assert_eq!(error.code, code, "{profile}: {error:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_moves_to_a_logged_in_profile_and_stays_moved() {
    within(async {
        let fixture = Fixture::new();
        let homes = tempfile::tempdir().unwrap();
        for home in ["alt", "work", "out"] {
            std::fs::create_dir_all(homes.path().join(home)).unwrap();
        }
        std::fs::write(
            fixture.data_dir().profiles_path(),
            profiles_toml(homes.path()),
        )
        .unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;

        let mut spec = structured_spec(&fixture, None);
        spec.profile = Some(name("claude-alt"));
        let id = client.create_session(spec).await.unwrap().id;
        let mut stream = client.follow(id, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| {
            status_of(record) == Some(&SessionStatus::Running)
        })
        .await;
        // Set to a mode other than the agent's default (protocol 9): the
        // agent restarted under the other profile must keep it.
        let plan = vessel_proto::OptionChoice {
            id: vessel_proto::OptionId::new("mode"),
            value: vessel_proto::ChoiceValue::Select {
                value: vessel_proto::OptionValueId::new("plan"),
            },
        };
        drop(client.set_option(id, plan).await.unwrap());
        let session = client.session(id).await.unwrap().harness_session.unwrap();
        let transcript = format!("{session}.jsonl");
        let project = claude_project(&homes.path().join("alt"), &fixture.cwd()).unwrap();
        std::fs::create_dir_all(project.join("memory")).unwrap();
        std::fs::write(project.join(&transcript), "{}\n").unwrap();
        std::fs::write(project.join("memory").join("MEMORY.md"), "kept\n").unwrap();

        // Refused, before anything is linked: a profile that is not logged
        // in, an unknown one, another harness's.
        refused(&client, id, "claude-out", ErrorCode::Conflict).await;
        refused(&client, id, "nope", ErrorCode::InvalidRequest).await;
        refused(&client, id, "codex-work", ErrorCode::InvalidRequest).await;
        assert!(!homes.path().join("out").join("projects").exists());
        assert_eq!(
            client.session(id).await.unwrap().profile,
            Some(name("claude-alt"))
        );

        let moved = client
            .move_profile(id, Some(name("claude-work")), false)
            .await
            .unwrap();
        assert_eq!(moved.profile, Some(name("claude-work")));
        assert_eq!(moved.status, SessionStatus::Running);
        read_until(&mut stream, &mut seen, |record| {
            matches!(record.event, SessionEvent::ProfileChanged { .. })
        })
        .await;
        assert_eq!(
            seen.last().unwrap().event,
            SessionEvent::ProfileChanged {
                from: Some(name("claude-alt")),
                to: Some(name("claude-work")),
            }
        );
        assert_eq!(
            ask(&client, &mut stream, &mut seen, id, "loaded").await,
            format!("loaded {session}")
        );
        assert!(
            ask(&client, &mut stream, &mut seen, id, "options")
                .await
                .contains("mode=plan"),
            "the restarted agent runs in the session's mode"
        );
        assert_eq!(
            ask(&client, &mut stream, &mut seen, id, "env CLAUDE_CONFIG_DIR").await,
            format!(
                "env CLAUDE_CONFIG_DIR={}",
                homes.path().join("work").display()
            )
        );
        let linked = claude_project(&homes.path().join("work"), &fixture.cwd()).unwrap();
        assert!(linked.join(&transcript).is_file());
        assert!(!linked.join("memory").exists(), "memory/ was linked");

        // A terminal session cannot move.
        let mut terminal = terminal_spec(&fixture, HarnessKind::ClaudeCode);
        terminal.profile = Some(name("claude-alt"));
        let terminal = client.create_session(terminal).await.unwrap().id;
        refused(&client, terminal, "claude-work", ErrorCode::Unsupported).await;
        client.kill(terminal).await.unwrap();

        // The session keeps the profile it moved to; ended, it cannot move.
        client.kill(id).await.unwrap();
        read_until(&mut stream, &mut seen, |record| {
            status_of(record).is_some_and(|status| !status.is_live())
        })
        .await;
        node.stop().await.unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let stored = client.session(id).await.unwrap();
        assert_eq!(stored.profile, Some(name("claude-work")));
        assert_eq!(stored.spec.profile, Some(name("claude-alt")));
        refused(&client, id, "claude-alt", ErrorCode::Conflict).await;
        node.stop().await.unwrap();
    })
    .await;
}

/// A move links the conversation's files into the target home, for good:
/// the session holds the conversation under every profile it went
/// through, live and when it starts again (from the moves its log
/// records), so no second agent runs on that file.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_moved_conversation_is_held_under_every_profile_it_went_through() {
    within(async {
        let fixture = Fixture::new();
        let homes = tempfile::tempdir().unwrap();
        for home in ["alt", "work", "third"] {
            std::fs::create_dir_all(homes.path().join(home)).unwrap();
        }
        std::fs::write(
            fixture.data_dir().profiles_path(),
            three_profiles(homes.path()),
        )
        .unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;

        let mut spec = structured_spec(&fixture, None);
        spec.profile = Some(name("claude-alt"));
        let id = client.create_session(spec).await.unwrap().id;
        until_status(&client, id, |status| *status == SessionStatus::Running).await;
        let conversation = client.session(id).await.unwrap().harness_session.unwrap();
        let project = claude_project(&homes.path().join("alt"), &fixture.cwd()).unwrap();
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(project.join(format!("{conversation}.jsonl")), "{}\n").unwrap();
        for to in ["claude-work", "claude-third"] {
            let moved = client
                .move_profile(id, Some(name(to)), false)
                .await
                .unwrap();
            assert_eq!(moved.profile, Some(name(to)));
        }

        // Live: refused under each, the first it left included.
        for profile in ["claude-alt", "claude-work", "claude-third"] {
            taken(
                client
                    .create_session(import(&fixture, &conversation, profile))
                    .await,
                id,
            );
        }

        // Ended by a restart, the store serves it. An import under the
        // profile it passed through runs; while it does, the session
        // cannot start again (neither its first profile nor its current
        // one is that one: the store read it from the moves).
        node.stop().await.unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let imported = client
            .create_session(import(&fixture, &conversation, "claude-work"))
            .await
            .unwrap()
            .id;
        until_status(&client, imported, |status| {
            *status == SessionStatus::Running
        })
        .await;
        taken(client.resume_session(id).await, imported);

        // Once the import has ended, it starts again, where it moved last.
        client.kill(imported).await.unwrap();
        until_status(&client, imported, |status| !status.is_live()).await;
        let resumed = client.resume_session(id).await.unwrap();
        assert_eq!(resumed.status, SessionStatus::Running);
        assert_eq!(resumed.profile, Some(name("claude-third")));
        node.stop().await.unwrap();
    })
    .await;
}
