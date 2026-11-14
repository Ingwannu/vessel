//! Protocol 12's parking over the wire (milestone 11, track A2, ADR 0038):
//! `session.park` and `session.wake`, the session's info and its records,
//! a prompt waking it, a wake that fails, an automatic prompt then waiting
//! in the queue, and the calls refused on an older connection.

use vessel_proto::parking::{ParkReason, ParkSession, WakeSession};
use vessel_proto::{
    ContentBlock, Delivered, Delivery, ErrorCode, EventSeq, SessionEvent, SessionId, SessionStatus,
};

use crate::agents::code;
use crate::support::{Fixture, ask, read_until, status_of, structured_spec, within};

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
