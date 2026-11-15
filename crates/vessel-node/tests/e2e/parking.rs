//! Protocol 12's parking over the wire (milestone 11, track A2, ADR 0038):
//! `session.park` and `session.wake`, the session's info and its records,
//! a prompt waking it, a wake that fails, an automatic prompt then waiting
//! in the queue, the calls refused on an older connection, and a silent
//! client dropped with what it follows.

use std::time::Duration;

use vessel_client::NodeClient;
use vessel_proto::parking::{ParkReason, ParkSession, WakeSession};
use vessel_proto::{
    ContentBlock, Delivered, Delivery, ErrorCode, EventSeq, SessionEvent, SessionId, SessionStatus,
};

use vessel_relay::Keepalive;

use crate::agents::code;
use crate::freezer::Freezer;
use crate::support::{Fixture, ask, client, read_until, status_of, step, structured_spec, within};

const PROFILES: &str = r#"
[profiles.breaks]
harness = "claude"
env = { FAKE_ACP_LOAD_FAILS = "1" }
"#;

fn text(text: &str) -> Vec<ContentBlock> {
    vec![ContentBlock::Text {
        text: text.to_owned(),
    }]
}

/// Parked by hand and woken by a prompt: the info says parked and running,
/// the log has `parked` then `woken`, and the conversation goes on.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_parked_session_is_woken_by_its_next_prompt() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let id = client
            .create_session(structured_spec(&fixture, None))
            .await
            .unwrap()
            .id;
        let mut stream = client.follow(id, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| {
            status_of(record) == Some(&SessionStatus::Running)
        })
        .await;
        assert_eq!(
            ask(&client, &mut stream, &mut seen, id, "plain").await,
            "echo: plain"
        );

        let parked = client.call(ParkSession { session: id }).await.unwrap();
        assert_eq!(parked.status, SessionStatus::Running);
        assert_eq!(
            parked.parked.map(|parked| parked.reason),
            Some(ParkReason::Asked)
        );
        let listed = client.sessions().await.unwrap();
        assert!(
            listed
                .iter()
                .any(|info| info.id == id && info.parked.is_some())
        );
        read_until(&mut stream, &mut seen, |record| {
            matches!(record.event, SessionEvent::Parked { .. })
        })
        .await;

        // A person's prompt wakes it (`session.prompt` and `session.send`
        // both go through the same delivery).
        assert_eq!(
            ask(&client, &mut stream, &mut seen, id, "loaded").await,
            format!(
                "loaded {}",
                client.session(id).await.unwrap().harness_session.unwrap()
            )
        );
        assert!(
            seen.iter()
                .any(|record| matches!(record.event, SessionEvent::Woken))
        );
        assert!(client.session(id).await.unwrap().parked.is_none());
        // Waking an awake session answers it as it is.
        let awake = client.call(WakeSession { session: id }).await.unwrap();
        assert!(awake.parked.is_none());
        assert_eq!(
            code(
                client
                    .call(ParkSession {
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

/// The agent cannot load its conversation any more: the wake fails
/// (`spawn_failed`), the session stays parked saying why, and a prompt
/// delivered `queue` (as the node's own are) waits in the queue, held,
/// instead of trying again.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_wake_stays_parked_and_queues_what_the_node_sends() {
    within(async {
        let fixture = Fixture::new();
        std::fs::write(fixture.data_dir().profiles_path(), PROFILES).unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let mut spec = structured_spec(&fixture, None);
        spec.profile = Some("breaks".parse().unwrap());
        let id = client.create_session(spec).await.unwrap().id;
        let mut stream = client.follow(id, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| {
            status_of(record) == Some(&SessionStatus::Running)
        })
        .await;
        let _parked = client.call(ParkSession { session: id }).await.unwrap();

        assert_eq!(
            code(client.call(WakeSession { session: id }).await),
            ErrorCode::SpawnFailed
        );
        let info = client.session(id).await.unwrap();
        assert_eq!(info.status, SessionStatus::Running);
        assert!(
            matches!(
                info.parked.map(|parked| parked.reason),
                Some(ParkReason::WakeFailed { .. })
            ),
            "it says why"
        );
        let queued = client
            .prompt(id, text("from a schedule"), Delivery::Queue)
            .await
            .unwrap();
        assert!(matches!(queued, Delivered::Queued { .. }), "{queued:?}");
        let conversation = client.conversation(id).await.unwrap();
        assert!(conversation.queue.held.is_some(), "held for a person");
        assert_eq!(conversation.queue.entries.len(), 1);
        // A person's prompt tries again (and fails again here).
        assert_eq!(
            code(client.prompt(id, text("plain"), Delivery::Start).await),
            ErrorCode::SpawnFailed
        );
        node.stop().await.unwrap();
    })
    .await;
}

/// A token client whose path went half-open (a laptop asleep, a network
/// gone) is dropped by the node's pings, as a `/secure` one is, and with it
/// the subscription it held. Until 2026-11-14 `/rpc` had no keepalive, so
/// such a client kept its subscriptions, and the sessions they follow,
/// for good. A quiet client that answers is kept. (Keepalive in
/// milliseconds here; 10 s and 30 s by default.)
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_silent_client_is_dropped_with_what_it_follows() {
    within(async {
        // Short, with room for a loaded machine.
        let fast = Keepalive {
            ping: Duration::from_millis(200),
            silence: Duration::from_secs(1),
        };
        let fixture = Fixture::new();
        let node = fixture.start_with(|config| config.keepalive = fast).await;
        let owner = fixture.connect(&node).await;
        let id = owner
            .create_session(structured_spec(&fixture, None))
            .await
            .unwrap()
            .id;
        let mut path = Freezer::start(node.address().unwrap()).await;
        let viewer = NodeClient::connect(&path.url(), &fixture.token(), client())
            .await
            .unwrap();
        let _watching = viewer.follow(id, EventSeq::ZERO).await.unwrap();
        // Quiet past the silence limit: the client answers the pings.
        tokio::time::sleep(fast.silence.saturating_mul(2)).await;
        assert!(
            path.closed_by_node.try_recv().is_err(),
            "a quiet client that answers was dropped"
        );
        path.freeze();
        let closed = step(
            "the node dropping the silent client",
            path.closed_by_node.recv(),
        )
        .await;
        assert!(closed.is_some(), "the node dropped the silent client");
        node.stop().await.unwrap();
    })
    .await;
}
