//! No agent process is needed to exercise admitted settings ownership.
use super::*;
use vessel_harness::{HarnessEnv, Lookup};
use vessel_proto::mcp::{Injection, McpSettingsPatch};
use vessel_session::{
    DEFAULT_DURABLE_CAPACITY, DEFAULT_HANDSHAKE_TIMEOUT, DEFAULT_RING_CAPACITY, RuntimeConfig,
};

const SETTINGS_ADMISSION_TEST_BUDGET: std::time::Duration = std::time::Duration::from_secs(30);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelling_a_settings_rpc_keeps_persistence_and_apply_tracked() {
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path().join("node"));
    data.prepare().unwrap();
    let stop = CancellationToken::new();
    let tasks = TaskTracker::new();
    let runtime = Arc::new(SessionRuntime::new(
        RuntimeConfig {
            harness_lookup: Lookup::new(Vec::new()),
            harness_env: HarnessEnv::new(root.path().join("home")),
            ring_capacity: DEFAULT_RING_CAPACITY,
            durable_capacity: DEFAULT_DURABLE_CAPACITY,
            agent_handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
            park_after: None,
            private_dir: None,
        },
        stop.clone(),
    ));
    let settings = Arc::new(Settings::open(
        data.clone(),
        runtime,
        ParkAfter::Never,
        false,
        tasks.clone(),
        stop.clone(),
    ));
    let writer_lock = settings.saved.lock().await;
    let request_settings = Arc::clone(&settings);
    let request = tokio::spawn(async move {
        request_settings
            .change(SetNodeSettings {
                park_after: None,
                drivers: None,
                mcp: Some(McpSettingsPatch {
                    inject: Some(Injection::On),
                    policy: None,
                }),
            })
            .await
    });
    tokio::time::timeout(SETTINGS_ADMISSION_TEST_BUDGET, async {
        while settings.changes.available_permits() == MAX_SETTINGS_CHANGES {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    request.abort();
    assert!(request.await.unwrap_err().is_cancelled());
    assert_eq!(
        settings.changes.available_permits(),
        MAX_SETTINGS_CHANGES - 1
    );
    assert!(!tasks.is_empty());
    drop(writer_lock);
    let _closed = tasks.close();
    tasks.wait().await;
    assert_eq!(settings.changes.available_permits(), MAX_SETTINGS_CHANGES);
    assert_eq!(
        data.read_settings().unwrap().unwrap().mcp.unwrap().inject,
        Some(Injection::On)
    );
    assert_eq!(settings.current().await.mcp.unwrap().inject, Injection::On);
    assert_eq!(settings.tool_policy().borrow().inject, Injection::On);
    stop.cancel();
}
