//! Log selection through the CLI with fake sessions and event-driven follow transitions.

use crate::support::{Captured, Fixture, within};
use std::time::Duration;

use vessel_client::NodeClient;
use vessel_proto::{
    AgentEvent, ContentBlock, EventRecord, EventSeq, HarnessKind, SessionEvent, SessionId,
    SessionInput, SessionMode, SessionSpec,
};

/// Each expected log transition has a deadline separate from the whole scenario.
const LOG_EVENT_BUDGET: Duration = Duration::from_secs(10);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tail_keeps_the_last_records_without_a_false_retention_notice_and_zero_is_empty() {
    within(async {
        let fixture = Fixture::start().await;
        let id = fixture.run_detached().await;
        let owner = fixture.owner().await;
        let session: SessionId = id.parse().unwrap();
        let attached = fixture
            .vessel(&["attach", &id], b"hello tail\rexit\r")
            .await;
        assert!(attached.ok, "{}", attached.stderr);
        let mut ended = owner.follow(session, EventSeq::ZERO).await.unwrap();
        tokio::time::timeout(LOG_EVENT_BUDGET, async {
            while let Some(record) = ended.next().await {
                drop(record.unwrap());
            }
        })
        .await
        .expect("the completed session's log did not close");
        let count = u32::try_from(owner.session(session).await.unwrap().next_seq.get()).unwrap();
        let all = owner.read(session, EventSeq::ZERO, count).await.unwrap();
        assert!(all.at_end);
        let last = all.records.last().unwrap().seq.to_string();
        let expected = fixture.vessel(&["log", &id, "--from", &last], b"").await;
        let tail = fixture.vessel(&["log", &id, "--tail", "1"], b"").await;
        assert!(
            expected.ok && tail.ok,
            "{} {}",
            expected.stderr,
            tail.stderr
        );
        assert_eq!(tail.stdout, expected.stdout);
        assert_eq!(tail.stderr, expected.stderr);
        assert!(!tail.stderr.contains("pruned"), "{}", tail.stderr);
        let zero = fixture.vessel(&["log", &id, "--tail", "0"], b"").await;
        assert!(zero.ok, "{}", zero.stderr);
        assert!(zero.stdout.is_empty() && zero.stderr.is_empty());
        fixture.stop().await;
    })
    .await;
}

/// A failed assertion must not leave the following CLI alive.
struct Following(tokio::process::Child);

impl Drop for Following {
    fn drop(&mut self) {
        let _already_ended = self.0.start_kill();
    }
}

async fn create(fixture: &Fixture, owner: &NodeClient) -> SessionId {
    let mut spec = SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    );
    spec.cwd = Some(fixture.data().to_path_buf());
    owner.create_session(spec).await.unwrap().id
}

async fn turn(owner: &NodeClient, session: SessionId, text: &str) -> EventSeq {
    let from = owner.session(session).await.unwrap().next_seq;
    owner
        .send(
            session,
            SessionInput::Prompt {
                content: vec![ContentBlock::Text {
                    text: text.to_owned(),
                }],
            },
        )
        .await
        .unwrap();
    let mut stream = owner.follow(session, from).await.unwrap();
    tokio::time::timeout(LOG_EVENT_BUDGET, async move {
        while let Some(record) = stream.next().await {
            if matches!(
                record.unwrap().event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnEnded { .. }
                }
            ) {
                return from;
            }
        }
        panic!("session ended before the fake completed its turn");
    })
    .await
    .expect("the fake turn did not finish")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn log_selects_time_kind_and_tail_without_reporting_filtered_records_as_pruned() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = fixture.owner().await;
        let session = create(&fixture, &owner).await;
        turn(&owner, session, "plain").await;
        let second = turn(&owner, session, "loaded").await;
        let id = session.to_string();
        let last = fixture
            .vessel(&["log", &id, "--filter", "text", "--tail", "1"], b"")
            .await;
        assert!(last.ok, "{}", last.stderr);
        assert_eq!(last.stdout, "loaded none");
        assert!(last.stderr.is_empty(), "{}", last.stderr);
        let page = owner.read(session, second, 100).await.unwrap();
        let first: &EventRecord = page.records.first().unwrap();
        let since = first.at.to_rfc3339();
        let selected = fixture
            .vessel(
                &[
                    "log",
                    &id,
                    "--from",
                    &second.to_string(),
                    "--since",
                    &since,
                    "--filter",
                    "text",
                ],
                b"",
            )
            .await;
        assert!(selected.ok, "{}", selected.stderr);
        assert!(!selected.stdout.contains("plain"), "{}", selected.stdout);
        assert!(
            selected.stdout.contains("loaded none"),
            "{}",
            selected.stdout
        );
        assert!(selected.stderr.is_empty(), "{}", selected.stderr);
        let zero = fixture.vessel(&["log", &id, "--tail", "0"], b"").await;
        assert!(zero.ok, "{}", zero.stderr);
        assert!(zero.stdout.is_empty() && zero.stderr.is_empty());
        let empty = fixture
            .vessel(&["log", &id, "--filter", "errors"], b"")
            .await;
        assert!(empty.ok, "{}", empty.stderr);
        assert!(
            empty.stdout.is_empty() && empty.stderr.is_empty(),
            "{} {}",
            empty.stdout,
            empty.stderr
        );
        owner.kill(session).await.unwrap();
        fixture.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn follow_keeps_the_history_cursor_and_prints_a_future_match_once() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = fixture.owner().await;
        let session = create(&fixture, &owner).await;
        turn(&owner, session, "plain").await;
        let id = session.to_string();
        let mut child = Following(fixture.spawn_vessel_at(
            &fixture.node_address(),
            &["log", &id, "--follow", "--filter", "text", "--tail", "1"],
        ));
        drop(child.0.stdin.take());
        let mut stdout = Captured::start(child.0.stdout.take().unwrap());
        let stderr = Captured::start(child.0.stderr.take().unwrap());
        stdout.wait_for("echo: plain", 1).await;
        // This can arrive between history and subscription: the saved cursor must catch it.
        turn(&owner, session, "loaded").await;
        stdout.wait_for("loaded none", 1).await;
        owner.kill(session).await.unwrap();
        assert!(child.0.wait().await.unwrap().success(), "{}", stderr.text());
        stdout.wait_for("loaded none\n", 1).await;
        let text = stdout.text();
        assert_eq!(text.matches("echo: plain").count(), 1, "{text}");
        assert_eq!(text.matches("loaded none").count(), 1, "{text}");
        assert!(text.contains("echo: plain\n> loaded"), "{text}");
        assert!(!stderr.text().contains("pruned"), "{}", stderr.text());
        fixture.stop().await;
    })
    .await;
}
