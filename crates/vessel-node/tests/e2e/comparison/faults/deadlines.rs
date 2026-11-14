//! Actual RPC effects with a controlled monotonic clock after the reply is withheld.
use super::*;

#[tokio::test]
async fn stalled_source_read_keeps_its_upload_and_ends_without_prompt() {
    clock(|| async {
        let fixture = Fixture::new();
        let node = clocked_node(&fixture).await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = crate::tidying::project(&owner).await;
        let bridge = Bridge::open(&fixture, &node).await;
        let job = prepared(&bridge.client, project.id, &["fake-small", "fake-large"]).await;
        let source = job.snapshot().targets[0].resources.session.unwrap();
        let file = attached(&owner, source, Bytes::from_static(b"stored bytes")).await;
        // The first read verifies the source's own input. Hold the next read,
        // after the second target's upload has actually begun on the real node.
        let arrival = bridge.hold_after(Method::ReadAttachment, 1).await;
        let sending = job.start(input("plain", Some((source, file))));
        tokio::pin!(sending);
        tokio::select! {
            reply = &mut sending => panic!("held read completed {reply:?}"),
            held = arrival => held.unwrap(),
        }
        tokio::time::pause();
        tokio::time::advance(std::time::Duration::from_secs(46)).await;
        let report = sending.await.unwrap();
        tokio::time::resume();
        let target = &report.targets[1];
        assert!(matches!(&target.status, TargetStatus::Failed(failure)
            if failure.phase == Phase::AttachmentRead && failure.kind == FailureKind::Deadline));
        let trail = target.resources.copy.as_ref().unwrap();
        assert!(trail.upload.is_some());
        assert_eq!(trail.acknowledged, 0);
        assert!(
            turns(&owner, target.resources.session.unwrap())
                .await
                .is_empty()
        );
        assert_eq!(bridge.count(Method::WriteUpload).await, 0);
        assert_eq!(turns(&owner, source).await.len(), 1);
        bridge.stop().await;
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test]
async fn a_lost_write_ack_expires_at_the_accepted_byte_progress_deadline() {
    clock(|| async {
        let fixture = Fixture::new();
        let node = clocked_node(&fixture).await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = crate::tidying::project(&owner).await;
        let bridge = Bridge::open(&fixture, &node).await;
        let job = prepared(&bridge.client, project.id, &["fake-small", "fake-large"]).await;
        let source = job.snapshot().targets[0].resources.session.unwrap();
        let file = attached(&owner, source, Bytes::from_static(b"stored bytes")).await;
        let arrival = bridge.hold(Method::WriteUpload).await;
        let sending = job.start(input("plain", Some((source, file))));
        tokio::pin!(sending);
        tokio::select! {
            reply = &mut sending => panic!("held write completed {reply:?}"),
            held = arrival => held.unwrap(),
        }
        tokio::time::pause();
        tokio::time::advance(std::time::Duration::from_secs(121)).await;
        let report = sending.await.unwrap();
        tokio::time::resume();
        let target = &report.targets[1];
        assert!(matches!(&target.status, TargetStatus::Uncertain(failure)
            if failure.phase == Phase::UploadWrite && failure.kind == FailureKind::Deadline));
        let trail = target.resources.copy.as_ref().unwrap();
        assert!(trail.upload.is_some());
        assert_eq!(trail.acknowledged, 0);
        assert_eq!(trail.attempted_bytes, 12);
        assert_eq!(bridge.count(Method::WriteUpload).await, 1);
        assert_eq!(bridge.count(Method::FinishUpload).await, 0);
        assert!(
            turns(&owner, target.resources.session.unwrap())
                .await
                .is_empty()
        );
        bridge.stop().await;
        node.stop().await.unwrap();
    })
    .await;
}

/// A node whose own `/rpc` keepalive outlasts these tests' clock jumps.
/// The node pings its clients and drops one silent for 30 s (ADR 0038's
/// amendment), and on the paused clock a jump of 46 s or 121 s is that
/// silence: the connections were dropped mid-test (gate 19). These tests
/// are about the client's own deadlines, not the node's keepalive, which
/// its own tests cover.
async fn clocked_node(fixture: &Fixture) -> vessel_node::Node {
    fixture
        .start_with(|config| {
            config.harness_lookup = crate::workspace::lookup(fixture);
            config.keepalive = vessel_relay::Keepalive {
                ping: std::time::Duration::from_secs(3600),
                silence: std::time::Duration::from_secs(3 * 3600),
            };
        })
        .await
}

// A separate current-thread runtime gives the operation its controlled clock.
// The existing outer 60 s safety net stays on real time; advancing a 120 s
// operation deadline must not alter or raise that test safety limit.
async fn clock<F: Future<Output = ()> + 'static>(make: impl FnOnce() -> F + Send + 'static) {
    within(async move {
        tokio::task::spawn_blocking(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(make());
        })
        .await
        .unwrap();
    })
    .await;
}
