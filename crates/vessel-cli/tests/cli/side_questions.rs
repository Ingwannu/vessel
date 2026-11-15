//! Side-question CLI routing through the real binary and fake agent.

use vessel_proto::SessionId;

use crate::support::{Fixture, within};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn side_command_opens_reuses_and_routes_later_questions() {
    within(async {
        let fixture = Fixture::start().await;
        let cwd = fixture.cwd();
        let parent = fixture
            .vessel(
                &["run", "claude", "--structured", "--detach", "--cwd", &cwd],
                b"",
            )
            .await;
        assert!(parent.ok, "{}", parent.stderr);
        let parent = parent.stdout.trim();
        let side = fixture.vessel(&["session", "side", parent], b"").await;
        assert!(side.ok, "{}", side.stderr);
        let side_id = side.stdout.trim();
        assert_ne!(side_id, parent);
        assert!(side_id.parse::<SessionId>().is_ok());
        let reused = fixture.vessel(&["session", "side", parent], b"").await;
        assert!(reused.ok, "{}", reused.stderr);
        assert_eq!(reused.stdout, side.stdout);
        let asked = fixture
            .vessel(&["session", "side", side_id, "What is happening?"], b"")
            .await;
        assert!(asked.ok, "{}", asked.stderr);
        assert_eq!(asked.stdout, side.stdout);
        let invalid = fixture
            .vessel(&["session", "side", side_id, "   "], b"")
            .await;
        assert!(!invalid.ok);
        let ordinary = fixture
            .vessel(&["send", side_id, "ordinary ingress"], b"")
            .await;
        assert!(!ordinary.ok);
        fixture.stop().await;
    })
    .await;
}
