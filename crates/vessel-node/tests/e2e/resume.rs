//! `session.resume` over the wire (protocol 10, milestone 11 track A, ADR
//! 0026): an ended session starts again with its log continued, its queue
//! held and its title kept, out of the archive; a subscription the store
//! was serving follows it; and every refusal the call can answer with
//! leaves the session as it was. Sessions end by a restart of the node,
//! after which the store serves them.

use vessel_harness::Lookup;
use vessel_proto::{
    AgentEvent, ContentBlock, Delivered, Delivery, ErrorCode, EventRecord, EventSeq, HarnessKind,
    KanbanColumn, Mark, QueueChange, QueueHold, SessionId, SessionInfo, SessionMode, SessionSpec,
    SessionStatus, StopWhen, SubscriptionEnd,
};
use vessel_rpc::SubscriptionItem;

use crate::agents::{as_agent, code, credential, following, structured, vessel_mcp};
use crate::support::{
    Fixture, agent_of, read_all, read_until, status_of, structured_spec, terminal_spec, within,
};

fn text(text: &str) -> Vec<ContentBlock> {
    vec![ContentBlock::Text {
        text: text.to_owned(),
    }]
}

/// The agent's message text in `records`.
fn said(records: &[EventRecord]) -> String {
    records
        .iter()
        .filter_map(|record| match agent_of(record) {
            Some(AgentEvent::MessageChunk {
                content: ContentBlock::Text { text },
            }) => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// Create a structured session as `spec` asks and wait until it runs.
async fn running(client: &vessel_client::NodeClient, spec: SessionSpec) -> SessionId {
    let id = client.create_session(spec).await.unwrap().id;
    let mut stream = client.follow(id, EventSeq::ZERO).await.unwrap();
    let mut seen = Vec::new();
    read_until(&mut stream, &mut seen, |record| {
        status_of(record) == Some(&SessionStatus::Running)
    })
    .await;
    id
}

/// An interrupted session starts again where its log ended: `starting`
/// right after its final status, the agent's replay left out, its title
/// kept, out of the archive and back in building; its queue held until a
/// person lets it move, and then its prompt is sent.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_ended_session_starts_again_where_its_log_ended() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let id = running(&client, structured_spec(&fixture, None)).await;
        assert_eq!(
            client
                .prompt(id, text("wait"), Delivery::Start)
                .await
                .unwrap(),
            Delivered::Started
        );
        let Delivered::Queued { entry } = client
            .prompt(id, text("plain"), Delivery::Queue)
            .await
            .unwrap()
        else {
            panic!("a turn runs: it waits");
        };
        node.stop().await.unwrap();

        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let before = client.session(id).await.unwrap();
        assert_eq!(before.status, SessionStatus::Interrupted);
        assert!(before.abilities.is_some_and(|abilities| abilities.resume));
        let archived = client.mark(id, Mark::Archive).await.unwrap();
        assert!(archived.marks.archived_at.is_some());

        let resumed = client.resume_session(id).await.unwrap();
        assert_eq!(resumed.id, id);
        assert_eq!(
            resumed.status,
            SessionStatus::Running,
            "answered once ready"
        );
        assert_eq!(resumed.title, before.title, "its title stays");
        assert_eq!(resumed.created_at, before.created_at);
        assert_eq!(resumed.created_by, before.created_by);
        assert_eq!(resumed.harness_session, before.harness_session);
        assert_eq!(resumed.marks.archived_at, None, "out of the archive");
        assert!(
            resumed
                .board
                .as_ref()
                .is_some_and(|board| board.column != KanbanColumn::Archive),
            "{:?}",
            resumed.board
        );
        assert!(resumed.next_seq > before.next_seq);
        let listed = client.sessions().await.unwrap();
        assert!(
            listed
                .iter()
                .any(|info| info.id == id && info.status == SessionStatus::Running)
        );

        let log = read_all(&client, id).await;
        let start = usize::try_from(before.next_seq.get()).unwrap();
        assert_eq!(log[start].seq, before.next_seq, "one log, no gap");
        assert_eq!(
            status_of(&log[start - 1]),
            Some(&SessionStatus::Interrupted)
        );
        assert_eq!(status_of(&log[start]), Some(&SessionStatus::Starting));
        assert!(
            !said(&log[start..]).contains("replayed"),
            "the agent's replay is not logged again"
        );
        let queue = client.conversation(id).await.unwrap().queue;
        assert_eq!(queue.held, Some(QueueHold::Ended), "back held");
        assert_eq!(
            queue
                .entries
                .iter()
                .map(|waiting| waiting.id)
                .collect::<Vec<_>>(),
            [entry]
        );

        let mut stream = client.follow(id, before.next_seq).await.unwrap();
        let mut seen = Vec::new();
        let moving = client.change_queue(id, QueueChange::Resume).await.unwrap();
        assert_eq!(moving.delivered, Some(Delivered::Started));
        read_until(&mut stream, &mut seen, |record| {
            matches!(agent_of(record), Some(AgentEvent::TurnEnded { .. }))
        })
        .await;
        assert_eq!(said(&seen), "echo: plain");
        assert_eq!(
            code(client.resume_session(id).await),
            ErrorCode::Conflict,
            "live already"
        );
        node.stop().await.unwrap();
    })
    .await;
}

/// A subscription the store was serving when the session started again
/// ends `lagged` from where it stopped, or, if it was done first,
/// `session_ended` at the old run's end; following on from there reaches
/// the new run either way, neither missing it nor looping.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stored_subscription_reaches_the_new_run() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let id = running(&client, structured_spec(&fixture, None)).await;
        node.stop().await.unwrap();

        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let before = client.session(id).await.unwrap();
        let mut stored = client.subscribe(id, EventSeq::ZERO).await.unwrap();
        let resumed = client.resume_session(id).await.unwrap();
        let mut got: Vec<EventRecord> = Vec::new();
        let end = loop {
            match stored.next().await {
                SubscriptionItem::Record(record) => got.push(record),
                SubscriptionItem::Ended(end) => break end,
                SubscriptionItem::Disconnected => panic!("the connection went"),
            }
        };
        let cursor = got.last().and_then(|record| record.seq.next()).unwrap();
        match end {
            SubscriptionEnd::Lagged { resume_from } => assert_eq!(resume_from, cursor),
            SubscriptionEnd::SessionEnded => {
                assert_eq!(cursor, before.next_seq, "everything the old run had");
            }
            SubscriptionEnd::Unsubscribed => panic!("nobody unsubscribed"),
        }
        let mut stream = client.follow(id, cursor).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| {
            record.seq >= before.next_seq && status_of(record) == Some(&SessionStatus::Running)
        })
        .await;
        assert!(seen.last().unwrap().seq < resumed.next_seq);
        client.kill(id).await.unwrap();
        read_until(&mut stream, &mut seen, |record| {
            status_of(record).is_some_and(|status| !status.is_live())
        })
        .await;
        assert!(stream.next().await.is_none(), "then the session's end");
        node.stop().await.unwrap();
    })
    .await;
}

/// Profiles that make the fake agent unable to resume, stop before the
/// handshake, or (`breaks`, once rewritten) fail to load a conversation.
const PROFILES: &str = r#"
[profiles.no-load]
harness = "claude"
env = { FAKE_ACP_NO_LOAD = "1" }

[profiles.exits]
harness = "claude"
env = { FAKE_ACP_EXIT_EARLY = "1" }

[profiles.gone]
harness = "claude"

[profiles.breaks]
harness = "claude"
"#;

/// The same, after the node's owner removed `gone` and broke `breaks`.
const PROFILES_AFTER: &str = r#"
[profiles.no-load]
harness = "claude"
env = { FAKE_ACP_NO_LOAD = "1" }

[profiles.exits]
harness = "claude"
env = { FAKE_ACP_EXIT_EARLY = "1" }

[profiles.breaks]
harness = "claude"
env = { FAKE_ACP_LOAD_FAILS = "1" }
"#;

fn under(fixture: &Fixture, profile: &str) -> SessionSpec {
    let mut spec = structured_spec(fixture, None);
    spec.profile = Some(profile.parse().unwrap());
    spec
}

/// Every refusal before an agent starts leaves the session as it was:
/// `not_found`, `conflict` for a live session, `unsupported` for a
/// terminal session, an agent that cannot resume and a session that never
/// had a conversation, `invalid_request` for a profile that is gone,
/// `forbidden` for an agent that does not own it, `unavailable` while the
/// node drains.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refusals_leave_the_session_as_it_was() {
    within(async {
        let fixture = Fixture::new();
        std::fs::write(fixture.data_dir().profiles_path(), PROFILES).unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let terminal = client
            .create_session(terminal_spec(&fixture, HarnessKind::ClaudeCode))
            .await
            .unwrap()
            .id;
        let no_load = running(&client, under(&fixture, "no-load")).await;
        let exits = client
            .create_session(under(&fixture, "exits"))
            .await
            .unwrap()
            .id;
        let gone = running(&client, under(&fixture, "gone")).await;
        let owners = running(&client, structured_spec(&fixture, None)).await;
        node.stop().await.unwrap();

        std::fs::write(fixture.data_dir().profiles_path(), PROFILES_AFTER).unwrap();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let client = fixture.connect(&node).await;
        let refused = |id: SessionId, expected: ErrorCode| {
            let client = &client;
            async move {
                let before = client.session(id).await.unwrap();
                assert_eq!(
                    code(client.resume_session(id).await),
                    expected,
                    "{:?}",
                    before.spec.profile
                );
                let after: SessionInfo = client.session(id).await.unwrap();
                assert_eq!(after.status, before.status, "still ended");
                assert_eq!(after.next_seq, before.next_seq, "nothing logged");
            }
        };
        assert_eq!(
            code(client.resume_session(SessionId::random()).await),
            ErrorCode::NotFound
        );
        refused(terminal, ErrorCode::Unsupported).await;
        let cannot = client.session(no_load).await.unwrap();
        assert!(cannot.abilities.is_some_and(|abilities| !abilities.resume));
        refused(no_load, ErrorCode::Unsupported).await;
        assert!(
            client
                .session(exits)
                .await
                .unwrap()
                .harness_session
                .is_none()
        );
        refused(exits, ErrorCode::Unsupported).await;
        refused(gone, ErrorCode::InvalidRequest).await;

        let mut parent = structured(&client, &fixture).await;
        assert_eq!(
            code(client.resume_session(parent.id).await),
            ErrorCode::Conflict,
            "live"
        );
        let (_program, socket, auth) = credential(&client, &mut parent).await;
        let agent = as_agent(&socket, &auth).await;
        assert_eq!(
            code(agent.resume_session(owners).await),
            ErrorCode::Forbidden,
            "the owner's, not the agent's"
        );

        client.stop_node(StopWhen::WhenIdle).await.unwrap();
        refused(owners, ErrorCode::Unavailable).await;
        client.kill(parent.id).await.unwrap();
        node.stop_requested().await;
        node.stop().await.unwrap();
    })
    .await;
}

/// An agent that cannot load the conversation: the call answers
/// `spawn_failed`, the attempt is in the log (`starting`, then `failed`
/// with the reason), and the session stays ended.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_conversation_that_does_not_load_leaves_the_attempt_in_the_log() {
    within(async {
        let fixture = Fixture::new();
        std::fs::write(fixture.data_dir().profiles_path(), PROFILES).unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let id = running(&client, under(&fixture, "breaks")).await;
        node.stop().await.unwrap();

        std::fs::write(fixture.data_dir().profiles_path(), PROFILES_AFTER).unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let before = client.session(id).await.unwrap();
        let Err(vessel_rpc::CallError::Remote(error)) = client.resume_session(id).await else {
            panic!("the load fails");
        };
        assert_eq!(error.code, ErrorCode::SpawnFailed);
        assert!(error.message.contains("session/load"), "{}", error.message);
        let mut stream = client.follow(id, before.next_seq).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| {
            status_of(record).is_some_and(|status| !status.is_live())
        })
        .await;
        assert_eq!(status_of(&seen[0]), Some(&SessionStatus::Starting));
        let Some(SessionStatus::Failed { message, .. }) = status_of(seen.last().unwrap()) else {
            panic!("it failed: {seen:?}");
        };
        assert!(message.contains("session/load"), "{message}");
        assert!(!client.session(id).await.unwrap().status.is_live());
        node.stop().await.unwrap();
    })
    .await;
}

/// A harness this node no longer has: `harness_not_installed`, before
/// anything starts.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_harness_no_longer_installed_is_refused() {
    within(async {
        let fixture = Fixture::with_gemini();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let spec = SessionSpec::new(
            HarnessKind::Gemini,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        )
        .with_cwd(fixture.cwd());
        let id = running(&client, spec).await;
        node.stop().await.unwrap();

        let empty = tempfile::tempdir().unwrap();
        let node = fixture
            .start_with(|config| config.harness_lookup = Lookup::new([empty.path().to_owned()]))
            .await;
        let client = fixture.connect(&node).await;
        let before = client.session(id).await.unwrap();
        assert_eq!(
            code(client.resume_session(id).await),
            ErrorCode::HarnessNotInstalled
        );
        assert_eq!(client.session(id).await.unwrap().next_seq, before.next_seq);
        node.stop().await.unwrap();
    })
    .await;
}

/// A session started again gets its agent a new credential, granted as a
/// session the owner creates would be (ADR 0013): its agent's `vessel`
/// server works, as itself, and the session's creator is still its own.
/// Its agent may ask questions, as a new session of its creator's would.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_started_again_gives_its_agent_a_new_credential() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let client = fixture.connect(&node).await;
        let mut first = structured(&client, &fixture).await;
        let (_program, _socket, old) = credential(&client, &mut first).await;
        node.stop().await.unwrap();

        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let client = fixture.connect(&node).await;
        let before = client.session(first.id).await.unwrap();
        let resumed = client.resume_session(first.id).await.unwrap();
        assert_eq!(resumed.created_by, before.created_by, "its creator stays");
        let mut again = following(&client, first.id, before.next_seq).await;
        let (_program, socket, new) = credential(&client, &mut again).await;
        assert_eq!(new.session, first.id);
        assert!(new.secret != old.secret, "a new one");
        let agent = as_agent(&socket, &new).await;
        let listed = agent.sessions().await.unwrap();
        assert!(listed.iter().any(|info| info.id == first.id));
        let log = read_all(&client, first.id).await;
        let start = usize::try_from(before.next_seq.get()).unwrap();
        assert!(
            log[start..].iter().any(|record| matches!(
                agent_of(record),
                Some(AgentEvent::HarnessLog { line }) if line.contains("the client takes questions")
            )),
            "the owner's session asks again"
        );
        node.stop().await.unwrap();
    })
    .await;
}
