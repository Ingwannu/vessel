//! Exercise real CLI output admission and the unchanged configured-model default.
use crate::support::{Fixture, within};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shared_output_process_modes_and_legacy_default() {
    within(async {
        let fixture = Fixture::start().await;
        let default = fixture.vessel(&["models"], b"").await;
        assert!(default.ok, "{}", default.stderr);
        assert_eq!(
            default.stdout,
            "no model routes (the node's models.toml defines them)\n"
        );
        let json = fixture.vessel(&["models", "--json"], b"").await;
        assert!(json.ok, "{}", json.stderr);
        let routes: Vec<vessel_proto::ModelRoute> = serde_json::from_str(&json.stdout).unwrap();
        assert!(routes.is_empty());
        let yaml = fixture.vessel(&["models", "-o", "yaml"], b"").await;
        assert!(yaml.ok, "{}", yaml.stderr);
        assert_eq!(yaml.stdout, "---\n[]\n");
        for flag in ["--quiet", "--no-headers"] {
            let selected = fixture.vessel(&["models", flag], b"").await;
            assert!(selected.ok, "{}", selected.stderr);
            assert!(selected.stdout.is_empty());
        }
        let status = fixture.vessel(&["status"], b"").await;
        assert!(status.ok, "{}", status.stderr);
        assert!(status.stdout.contains("claude-code"));
        for args in [["sessions", "--json"], ["status", "--quiet"]] {
            let refused = fixture.vessel(&args, b"").await;
            assert!(!refused.ok);
            assert!(refused.stdout.is_empty());
            assert!(refused.stderr.contains("does not support explicit output"));
        }
        fixture.stop().await;
    })
    .await;
}
