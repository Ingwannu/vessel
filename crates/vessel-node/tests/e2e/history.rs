//! Protocol 17's `agent.history` (track A2, ADR 0053), with `fake-acp`,
//! which keeps three earlier conversations over two pages (and one in a
//! relative folder, left out): a listing pages through them, an import
//! (`session.create` with `resume`) replays one into a new session's log
//! and marks its entry, a second import of it is refused while the first
//! runs, a long conversation imports within the handshake, and the
//! refusals, a device paired with `read` alone among them. A ranked
//! listing (ADR 0109) answers the conversations not imported with their
//! prompts, counts the imported one, and searches on the node.

use std::time::Instant;

use vessel_client::Connector as _;
use vessel_client::secure::{RouteTimeouts, pair_over};
use vessel_proto::history::{AgentHistoryPage, GetAgentHistory};
use vessel_proto::{
    AgentEvent, ContentBlock, ErrorCode, EventSeq, HarnessKind, HarnessRef, HarnessSessionId,
    Scope, SessionEvent, SessionId, SessionStatus,
};

use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::secure::{Relay, device_via, dialer, relayed_with};
use crate::support::{Fixture, client, read_until, status_of, structured_spec, within};

const PROFILES: &str = r#"
[profiles.cannot]
harness = "claude"
env = { FAKE_ACP_NO_LIST = "1" }

[profiles.long]
harness = "claude"
env = { FAKE_ACP_REPLAY = "10000" }

[profiles.lastly]
harness = "claude"
env = { FAKE_ACP_REPLAY_USER = "and   ship\nit" }
"#;

fn asked(fixture: &Fixture, cursor: Option<String>) -> GetAgentHistory {
    GetAgentHistory {
        harness: HarnessRef::Known(HarnessKind::ClaudeCode),
        profile: None,
        cwd: Some(fixture.cwd()),
        cursor,
        limit: None,
        query: None,
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

fn ranked(fixture: &Fixture, limit: u16, query: Option<&str>) -> GetAgentHistory {
    GetAgentHistory {
        limit: Some(limit),
        query: query.map(str::to_owned),
        ..asked(fixture, None)
    }
}

/// ADR 0109: `limit` answers the newest conversations not on the node,
/// newest first, each with the prompts its load replayed (`fake-acp`
/// replays `earlier question`, and with `FAKE_ACP_REPLAY_USER` a later
/// one), with no `next`; an imported one is left out and counted; a query
/// matches titles and prompts on the node; the ranked fields are refused
/// with a cursor, and a query without a limit.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_ranked_listing_answers_prompts_leaves_out_imports_and_searches() {
    within(async {
        let fixture = Fixture::new();
        std::fs::write(fixture.data_dir().profiles_path(), PROFILES).unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;

        let all = client.call(ranked(&fixture, 15, None)).await.unwrap();
        assert_eq!(ids(&all), ["past-3", "past-2", "past-1"]);
        assert_eq!(all.next, None);
        assert_eq!(all.already_imported, Some(0));
        assert!(all.conversations.iter().all(|entry| {
            entry.first_prompt.as_deref() == Some("earlier question")
                && entry.last_prompt.as_deref() == Some("earlier question")
                && entry.in_vessel.is_none()
        }));

        let mut lastly = ranked(&fixture, 1, None);
        lastly.profile = Some("lastly".parse().unwrap());
        let lastly = client.call(lastly).await.unwrap();
        assert_eq!(ids(&lastly), ["past-3"], "a limit of one");
        assert_eq!(
            lastly.conversations[0].last_prompt.as_deref(),
            Some("and ship it"),
            "the last prompt, its blanks folded"
        );

        // Imported, past-2 is left out and counted.
        let mut spec = structured_spec(&fixture, Some(HarnessSessionId::new("past-2")));
        spec.cwd = Some(all.conversations[1].cwd.clone());
        let imported = client.create_session(spec).await.unwrap();
        let _seen = until_running(&client, imported.id).await;
        let after = client.call(ranked(&fixture, 15, None)).await.unwrap();
        assert_eq!(ids(&after), ["past-3", "past-1"]);
        assert_eq!(after.already_imported, Some(1));

        // Searched on the node: a title, a prompt (letter case aside).
        let titled = client
            .call(ranked(&fixture, 15, Some("  THIRD ")))
            .await
            .unwrap();
        assert_eq!(ids(&titled), ["past-3"]);
        let prompted = client
            .call(ranked(&fixture, 15, Some("Earlier")))
            .await
            .unwrap();
        assert_eq!(ids(&prompted), ["past-3", "past-1"]);
        assert_eq!(prompted.already_imported, Some(1), "past-2 matches too");
        let unmatched = client
            .call(ranked(&fixture, 15, Some("nothing like it")))
            .await
            .unwrap();
        assert!(unmatched.conversations.is_empty());
        assert_eq!(unmatched.already_imported, Some(0));

        // Refusals.
        let mut paged = ranked(&fixture, 15, None);
        paged.cursor = Some("0~".to_owned());
        assert_eq!(code(client.call(paged).await), ErrorCode::InvalidRequest);
        let mut unlimited = asked(&fixture, None);
        unlimited.query = Some("third".to_owned());
        assert_eq!(
            code(client.call(unlimited).await),
            ErrorCode::InvalidRequest
        );
        assert_eq!(
            code(client.call(ranked(&fixture, 0, None)).await),
            ErrorCode::InvalidRequest
        );
        let mut cannot = ranked(&fixture, 15, None);
        cannot.profile = Some("cannot".parse().unwrap());
        assert_eq!(code(client.call(cannot).await), ErrorCode::Unsupported);
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

/// Listing an agent's conversations reads its files and leads to starting
/// one (`operate`, ADR 0053): a device paired with `read` alone is
/// refused.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_read_only_device_is_refused_the_listing() {
    within(async {
        let relay = Relay::start().await;
        let fixture = Fixture::new();
        let node = relayed_with(&fixture, &[&relay.url], |_config| {}).await;
        let owner = fixture.connect(&node).await;
        let offer = owner
            .invite_with_scopes(None, 600, vec![Scope::Read])
            .await
            .unwrap()
            .link
            .decode()
            .unwrap();
        let (paired, _route) = pair_over(&dialer(), &offer, "reader", RouteTimeouts::default())
            .await
            .unwrap();
        let reader = device_via(dialer(), &paired, &[], &[&relay.url])
            .connect(client())
            .await
            .unwrap();
        assert_eq!(
            code(reader.call(asked(&fixture, None)).await),
            ErrorCode::Forbidden
        );
        assert!(owner.call(asked(&fixture, None)).await.is_ok());
    })
    .await;
}
