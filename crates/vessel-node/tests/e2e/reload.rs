//! `session.reload` over the wire (TRACKER 803, ADR 0110): the agent
//! started again on the same conversation, what its own history adds to
//! the log (the coordinator's ruling C: nothing new, turns made outside
//! Vessel added once, a history that does not begin with the log adding
//! nothing), an archived session unarchived and started again, a turn that
//! does not end kept, and the refusals.
//!
//! `fake-acp` replays a conversation whose first prompt is
//! `FAKE_ACP_REPLAY_FIRST` (`earlier question` without it), then, with
//! `FAKE_ACP_REPLAY_USER`, a later prompt nobody sent through Vessel.

use vessel_client::NodeClient;
use vessel_proto::reload::ReloadSession;
use vessel_proto::{
    AgentEvent, ContentBlock, ErrorCode, EventRecord, EventSeq, HarnessKind, Mark, SessionEvent,
    SessionId, SessionStatus,
};

use crate::agents::{as_agent, code, credential, following, structured, vessel_mcp};
use crate::support::{Fixture, ask, prompt, read_until, terminal_spec, within};

const PROFILES: &str = r#"
[profiles.lined]
harness = "claude"
env = { FAKE_ACP_REPLAY_FIRST = "plain" }

[profiles.outside]
harness = "claude"
env = { FAKE_ACP_REPLAY_FIRST = "plain", FAKE_ACP_REPLAY_USER = "made outside" }
"#;

/// A running session under `profile` (the node's default when `None`)
/// that was asked `plain` once.
async fn asked(
    client: &NodeClient,
    fixture: &Fixture,
    profile: Option<&str>,
) -> crate::agents::Running {
    let mut spec = crate::support::structured_spec(fixture, None);
    spec.profile = profile.map(|name| name.parse().unwrap());
    let id = client.create_session(spec).await.unwrap().id;
    let mut running = following(client, id, EventSeq::ZERO).await;
    let said = ask(client, &mut running.stream, &mut running.seen, id, "plain").await;
    assert_eq!(said, "echo: plain");
    running
}

/// Reload `running`'s session and read its log through the `reloaded`
/// record and the `appended` records after it; returns those, and the line
/// that may follow them.
async fn reload(
    client: &NodeClient,
    running: &mut crate::agents::Running,
) -> (u32, Vec<EventRecord>) {
    let before = client.session(running.id).await.unwrap();
    let info = client
        .call(ReloadSession {
            session: running.id,
        })
        .await
        .unwrap();
    assert_eq!(info.status, SessionStatus::Running);
    assert!(info.parked.is_none());
    assert_eq!(
        info.harness_session, before.harness_session,
        "same conversation"
    );
    let start = running.seen.len();
    read_until(&mut running.stream, &mut running.seen, |record| {
        matches!(record.event, SessionEvent::Reloaded { .. })
    })
    .await;
    let Some(SessionEvent::Reloaded {
        harness_session,
        appended,
    }) = running.seen.last().map(|record| record.event.clone())
    else {
        panic!("read until `reloaded`");
    };
    assert_eq!(Some(harness_session), before.harness_session);
    // The reply's `next_seq` (Paseo's `timelineSize`) covers it.
    let at = running.seen.last().map(|record| record.seq).unwrap();
    assert!(info.next_seq > at);
    let mut after = Vec::new();
    for _ in 0..appended {
        let record = running.stream.next().await.unwrap().unwrap();
        after.push(record.clone());
        running.seen.push(record);
    }
    assert!(
        running.seen[start..]
            .iter()
            .all(|record| !matches!(record.event, SessionEvent::Status { .. })),
        "a live reload changes no status"
    );
    (appended, after)
}

fn user_text(record: &EventRecord) -> Option<&str> {
    if let SessionEvent::Agent {
        event:
            AgentEvent::UserMessageChunk {
                content: ContentBlock::Text { text },
            },
    } = &record.event
    {
        Some(text)
    } else {
        None
    }
}

fn lines(seen: &[EventRecord]) -> Vec<&str> {
    seen.iter()
        .filter_map(|record| {
            if let SessionEvent::Agent {
                event: AgentEvent::HarnessLog { line },
            } = &record.event
            {
                Some(line.as_str())
            } else {
                None
            }
        })
        .collect()
}

/// Nothing new: the log has the history's prompts, so `reloaded` adds
/// nothing and says nothing; the agent was started with `session/load` on
/// the same conversation and goes on.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reload_with_nothing_new_keeps_the_conversation() {
    within(async {
        let fixture = Fixture::new();
        std::fs::write(fixture.data_dir().profiles_path(), PROFILES).unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let mut running = asked(&client, &fixture, Some("lined")).await;
        let (appended, _) = reload(&client, &mut running).await;
        assert_eq!(appended, 0);
        assert!(
            !lines(&running.seen)
                .iter()
                .any(|line| line.contains("own history")),
            "{:?}",
            lines(&running.seen)
        );
        let conversation = client
            .session(running.id)
            .await
            .unwrap()
            .harness_session
            .unwrap();
        let said = ask(
            &client,
            &mut running.stream,
            &mut running.seen,
            running.id,
            "loaded",
        )
        .await;
        assert_eq!(said, format!("loaded {conversation}"));
        node.stop().await.unwrap();
    })
    .await;
}

/// A prompt made outside Vessel is added after `reloaded`, once: a second
/// reload finds the log's prompts are the history's and adds nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn turns_made_outside_vessel_are_added_once() {
    within(async {
        let fixture = Fixture::new();
        std::fs::write(fixture.data_dir().profiles_path(), PROFILES).unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let mut running = asked(&client, &fixture, Some("outside")).await;
        let (appended, added) = reload(&client, &mut running).await;
        assert_eq!(appended, 1);
        assert_eq!(
            added.iter().filter_map(user_text).collect::<Vec<_>>(),
            ["made outside"]
        );
        let (again, _) = reload(&client, &mut running).await;
        assert_eq!(again, 0, "no duplicate on a second reload");
        assert_eq!(
            running
                .seen
                .iter()
                .filter_map(user_text)
                .filter(|text| *text == "made outside")
                .count(),
            1
        );
        node.stop().await.unwrap();
    })
    .await;
}

/// The history does not begin with the log's prompt (`earlier question`
/// against `plain`): nothing is added, and one line says why.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_history_that_does_not_begin_with_the_log_adds_nothing() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let mut running = asked(&client, &fixture, None).await;
        let (appended, _) = reload(&client, &mut running).await;
        assert_eq!(appended, 0);
        read_until(&mut running.stream, &mut running.seen, |record| {
            matches!(
                &record.event,
                SessionEvent::Agent { event: AgentEvent::HarnessLog { line } }
                    if line.contains("does not begin with this log's prompts")
            )
        })
        .await;
        node.stop().await.unwrap();
    })
    .await;
}

/// An archived session (ended by its archiving) is unarchived and started
/// again on its conversation, with `reloaded` in its new run.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_archived_session_is_unarchived_and_started_again() {
    within(async {
        let fixture = Fixture::new();
        std::fs::write(fixture.data_dir().profiles_path(), PROFILES).unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let running = asked(&client, &fixture, Some("lined")).await;
        let archived = client.mark(running.id, Mark::Archive).await.unwrap();
        assert!(archived.marks.archived_at.is_some());
        let ended = client.session(running.id).await.unwrap();
        assert!(!ended.status.is_live());
        let info = client
            .call(ReloadSession {
                session: running.id,
            })
            .await
            .unwrap();
        assert_eq!(info.status, SessionStatus::Running);
        assert!(info.marks.archived_at.is_none(), "unarchived");
        assert_eq!(info.harness_session, ended.harness_session);
        let mut again = client.follow(running.id, ended.next_seq).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut again, &mut seen, |record| {
            matches!(record.event, SessionEvent::Reloaded { appended: 0, .. })
        })
        .await;
        node.stop().await.unwrap();
    })
    .await;
}

/// A turn that does not end on its cancel is kept: the reload is refused
/// (`conflict`), nothing was started, and the turn goes on.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_turn_that_does_not_end_is_kept() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let mut running = structured(&client, &fixture).await;
        client.send(running.id, prompt("deaf")).await.unwrap();
        read_until(&mut running.stream, &mut running.seen, |record| {
            matches!(
                &record.event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnStarted { .. }
                }
            )
        })
        .await;
        let before = client.session(running.id).await.unwrap();
        assert_eq!(
            code(
                client
                    .call(ReloadSession {
                        session: running.id
                    })
                    .await
            ),
            ErrorCode::Conflict
        );
        let info = client.session(running.id).await.unwrap();
        assert_eq!(info.status, SessionStatus::Running);
        assert_eq!(info.harness_session, before.harness_session);
        let page = client.read(running.id, EventSeq::ZERO, 1000).await.unwrap();
        assert!(
            !page.records.iter().any(|record| matches!(
                record.event,
                SessionEvent::Reloaded { .. }
                    | SessionEvent::Agent {
                        event: AgentEvent::TurnEnded { .. }
                    }
            )),
            "no reload, and the turn still runs"
        );
        node.stop().await.unwrap();
    })
    .await;
}

/// A person reloads: an agent is `forbidden`; a terminal session is
/// `unsupported`; an unknown one `not_found`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_and_a_terminal_are_refused() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let owner = fixture.connect(&node).await;
        let mut parent = structured(&owner, &fixture).await;
        let (_program, socket, auth) = credential(&owner, &mut parent).await;
        let agent = as_agent(&socket, &auth).await;
        assert_eq!(
            code(agent.call(ReloadSession { session: parent.id }).await),
            ErrorCode::Forbidden
        );
        let terminal = owner
            .create_session(terminal_spec(&fixture, HarnessKind::ClaudeCode))
            .await
            .unwrap()
            .id;
        assert_eq!(
            code(owner.call(ReloadSession { session: terminal }).await),
            ErrorCode::Unsupported
        );
        assert_eq!(
            code(
                owner
                    .call(ReloadSession {
                        session: SessionId::random()
                    })
                    .await
            ),
            ErrorCode::NotFound
        );
        node.stop().await.unwrap();
    })
    .await;
}
