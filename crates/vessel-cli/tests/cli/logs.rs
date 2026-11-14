//! Tail selection through the real CLI after a fake terminal's final record.

use crate::support::{Fixture, within};
use vessel_proto::{EventSeq, SessionId};

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
        while let Some(record) = ended.next().await {
            drop(record.unwrap());
        }
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
