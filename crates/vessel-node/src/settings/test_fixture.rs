//! Empty session runtime for settings tests; no agent, process environment or node.
use super::*;

/// Registry and settings cases share the real owner with explicit temporary homes.
pub(crate) fn open(data: DataDir) -> Arc<Settings> {
    let runtime = Arc::new(SessionRuntime::new(
        vessel_session::RuntimeConfig {
            harness_lookup: vessel_harness::Lookup::new(std::iter::empty::<std::path::PathBuf>()),
            harness_env: vessel_harness::HarnessEnv::new(data.root().to_owned()),
            ring_capacity: vessel_session::DEFAULT_RING_CAPACITY,
            durable_capacity: vessel_session::DEFAULT_DURABLE_CAPACITY,
            agent_handshake_timeout: vessel_session::DEFAULT_HANDSHAKE_TIMEOUT,
            park_after: None,
            private_dir: None,
        },
        tokio_util::sync::CancellationToken::new(),
    ));
    Arc::new(Settings::open(data, runtime, ParkAfter::DEFAULT, false))
}
