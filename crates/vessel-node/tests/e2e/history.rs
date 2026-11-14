//! Protocol 17's `agent.history` (track A2, ADR 0053), with `fake-acp`,
//! which keeps three earlier conversations over two pages (and one in a
//! relative folder, left out): a listing pages through them, an import
//! (`session.create` with `resume`) replays one into a new session's log
//! and marks its entry, a second import of it is refused while the first
//! runs, a long conversation imports within the handshake, and the
//! refusals.

use std::time::Instant;

use vessel_proto::history::{AgentHistoryPage, GetAgentHistory};
use vessel_proto::{
    AgentEvent, ContentBlock, ErrorCode, EventSeq, HarnessKind, HarnessRef, HarnessSessionId,
    SessionEvent, SessionId, SessionStatus,
};

use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::support::{Fixture, read_until, status_of, structured_spec, within};

const PROFILES: &str = r#"
[profiles.cannot]
harness = "claude"
env = { FAKE_ACP_NO_LIST = "1" }

[profiles.long]
harness = "claude"
env = { FAKE_ACP_REPLAY = "10000" }
"#;

fn asked(fixture: &Fixture, cursor: Option<String>) -> GetAgentHistory {
    GetAgentHistory {
        harness: HarnessRef::Known(HarnessKind::ClaudeCode),
        profile: None,
        cwd: Some(fixture.cwd()),
        cursor,
    }
}

fn ids(page: &AgentHistoryPage) -> Vec<&str> {
    page.conversations
        .iter()
        .map(|entry| entry.harness_session.as_str())
        .collect()
}

/// The text of the agent message chunks in `records`.
fn said(records: &[vessel_proto::EventRecord]) -> Vec<&str> {
    records
        .iter()
        .filter_map(|record| {
            let SessionEvent::Agent {
                event:
                    AgentEvent::MessageChunk {
                        content: ContentBlock::Text { text },
                    },
            } = &record.event
            else {
                return None;
            };
            Some(text.as_str())
        })
        .collect()
}

/// The new session `id`, followed until it runs; what it logged.
async fn until_running(
    client: &vessel_client::NodeClient,
    id: SessionId,
) -> Vec<vessel_proto::EventRecord> {
    let mut stream = client.follow(id, EventSeq::ZERO).await.unwrap();
    let mut seen = Vec::new();
    read_until(&mut stream, &mut seen, |record| {
        status_of(record) == Some(&SessionStatus::Running)
    })
    .await;
    seen
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_listing_pages_and_an_import_replays_and_marks_its_entry() {
    within(async {
        let fixture = Fixture::new();
        std::fs::write(fixture.data_dir().profiles_path(), PROFILES).unwrap();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let client = fixture.connect(&node).await;
        let folder = std::fs::canonicalize(fixture.cwd()).unwrap();

        // Two pages; the entry in a relative folder is left out.
        let first = client.call(asked(&fixture, None)).await.unwrap();
        assert_eq!(ids(&first), ["past-3", "past-2"]);
        assert!(first.conversations.iter().all(|entry| entry.cwd == folder));
        assert!(
            first
                .conversations
                .iter()
                .all(|entry| entry.in_vessel.is_none())
        );
        let third = &first.conversations[0];
        assert_eq!(
            third.title.as_ref().map(ToString::to_string),
            Some("third".to_owned())
        );
        assert_eq!(
            third.updated_at.map(|at| at.to_rfc3339()),
            Some("2026-11-03T09:00:00+00:00".to_owned())
        );
        let second = client
            .call(asked(&fixture, first.next.clone()))
            .await
            .unwrap();
        assert_eq!(ids(&second), ["past-1"]);
        assert_eq!(second.next, None);
        assert_eq!(second.conversations[0].title, None);
        assert_eq!(second.conversations[0].updated_at, None);

        // An import: `session.create` resuming the entry, in its folder,
        // with its title; the conversation is replayed into the new log.
        let entry = &first.conversations[1];
        let mut spec = structured_spec(&fixture, Some(entry.harness_session.clone()));
        spec.cwd = Some(entry.cwd.clone());
        spec.title = entry.title.clone();
        let imported = client.create_session(spec.clone()).await.unwrap();
        assert_eq!(imported.title, entry.title);
        let seen = until_running(&client, imported.id).await;
        assert!(said(&seen).contains(&"replayed past-2"), "{seen:#?}");
        assert!(
            seen.iter().any(|record| matches!(
                &record.event,
                SessionEvent::Agent {
                    event: AgentEvent::UserMessageChunk { .. }
                }
            )),
            "{seen:#?}"
        );

        // Listed again, the entry names the session on it.
        let again = client.call(asked(&fixture, None)).await.unwrap();
        let marked: Vec<_> = again
            .conversations
            .iter()
            .map(|entry| (entry.harness_session.as_str(), entry.in_vessel))
            .collect();
        assert_eq!(marked, [("past-3", None), ("past-2", Some(imported.id))]);

        // A second agent on that conversation is refused while it runs.
        assert_eq!(code(client.create_session(spec).await), ErrorCode::Conflict);

        // Refusals: a cursor no listing gave, a relative folder, an agent
        // that does not list, and an agent.
        assert_eq!(
            code(client.call(asked(&fixture, Some("x".to_owned()))).await),
            ErrorCode::InvalidRequest
        );
        let mut relative = asked(&fixture, None);
        relative.cwd = Some("work".into());
        assert_eq!(code(client.call(relative).await), ErrorCode::InvalidRequest);
        let mut cannot = asked(&fixture, None);
        cannot.profile = Some("cannot".parse().unwrap());
        assert_eq!(code(client.call(cannot).await), ErrorCode::Unsupported);
        let mut running = structured(&client, &fixture).await;
        let (_program, socket, auth) = credential(&client, &mut running).await;
        let agent = as_agent(&socket, &auth).await;
        assert_eq!(
            code(agent.call(asked(&fixture, None)).await),
            ErrorCode::Forbidden
        );
    })
    .await;
}

/// An import replays the whole conversation inside the agent's handshake
/// (its 60 s): ten thousand replayed messages, each a durable record.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_long_conversation_imports_within_the_handshake() {
    within(async {
        let fixture = Fixture::new();
        std::fs::write(fixture.data_dir().profiles_path(), PROFILES).unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let mut spec = structured_spec(&fixture, Some(HarnessSessionId::new("past-1")));
        spec.profile = Some("long".parse().unwrap());
        let started = Instant::now();
        let imported = client.create_session(spec).await.unwrap();
        let seen = until_running(&client, imported.id).await;
        let replayed = said(&seen)
            .into_iter()
            .filter(|text| text.starts_with("replayed line "))
            .count();
        // In the test's log (`support::logs`), for the plan's figure.
        tracing::info!(
            replayed,
            ms = started.elapsed().as_millis(),
            "a long conversation imported"
        );
        assert_eq!(replayed, 10_000);
    })
    .await;
}
