//! Selected-node pending diagnostics are explicit refusals, not local reports.
use crate::support::{Fixture, within};
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn node_diagnostics_uses_the_selected_node_and_suppresses_private_debug_output() {
    within(async {
        let private = "PRIVATE-DIAGNOSTIC-CLI-CANARY";
        let fixture = Fixture::start_with(|config, _bin| {
            config.name = Some(private.to_owned());
        })
        .await;
        for args in [
            vec!["node", "diagnostics"],
            vec!["node", "diagnostics", "--json"],
        ] {
            let result = fixture.vessel_debug(&args, b"").await;
            assert!(!result.ok);
            assert!(result.stdout.is_empty());
            assert!(result.stderr.contains("protocol number is not assigned"));
            assert!(!result.stderr.contains(private));
            assert!(!result.stderr.contains("client connected"));
        }
        fixture.stop().await;
    })
    .await;
}
