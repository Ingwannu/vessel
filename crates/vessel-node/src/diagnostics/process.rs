//! Diagnostic source used by the bounded node collector (ADR 0074).
//! Source owners omit private paths, arbitrary text and credentials.
use crate::lifecycle::Lifecycle;
use tokio::time::Instant;
use vessel_proto::{
    DurationMs,
    diagnostics::{DiagnosticArch, DiagnosticHome, DiagnosticNode, DiagnosticOs},
};

pub(super) fn observation(lifecycle: &Lifecycle, started: Instant) -> DiagnosticNode {
    let source = lifecycle.watch_info();
    let node = source.borrow();
    DiagnosticNode {
        id: node.id,
        state: node.state,
        os: platform(),
        arch: architecture(),
        version: env!("CARGO_PKG_VERSION").parse().ok(),
        uptime: DurationMs::from_duration(started.elapsed()),
        home: DiagnosticHome::Redacted,
    }
}
fn platform() -> DiagnosticOs {
    if cfg!(target_os = "linux") {
        DiagnosticOs::Linux
    } else if cfg!(target_os = "macos") {
        DiagnosticOs::MacOs
    } else if cfg!(target_os = "windows") {
        DiagnosticOs::Windows
    } else {
        DiagnosticOs::Other
    }
}
fn architecture() -> DiagnosticArch {
    if cfg!(target_arch = "x86_64") {
        DiagnosticArch::X86_64
    } else if cfg!(target_arch = "aarch64") {
        DiagnosticArch::Aarch64
    } else if cfg!(target_arch = "x86") {
        DiagnosticArch::X86
    } else if cfg!(target_arch = "arm") {
        DiagnosticArch::Arm
    } else {
        DiagnosticArch::Other
    }
}

#[cfg(test)]
mod tests {
    use super::super::deadline::{self, phase};
    use super::*;
    use chrono::DateTime;
    use std::{sync::Arc, time::Duration};
    use tokio_util::sync::CancellationToken;
    use vessel_harness::{HarnessEnv, Lookup};
    use vessel_proto::{NodeId, NodeInfo, NodeState, ProtocolRange, StopWhen};
    use vessel_secure::NodeKeypair;
    use vessel_session::{
        DEFAULT_DURABLE_CAPACITY, DEFAULT_HANDSHAKE_TIMEOUT, DEFAULT_RING_CAPACITY, RuntimeConfig,
        SessionRuntime,
    };

    const OBSERVATION_GAP: Duration = Duration::from_secs(3);
    #[tokio::test(start_paused = true)]
    async fn borrowing_lifecycle_drops_private_text_and_uses_the_monotonic_clock() {
        deadline::run(async {
            let temp = tempfile::tempdir().unwrap();
            let stop = CancellationToken::new();
            let runtime = Arc::new(SessionRuntime::new(
                RuntimeConfig {
                    harness_lookup: Lookup::new([temp.path().to_owned()]),
                    harness_env: HarnessEnv::new(temp.path().to_owned()),
                    ring_capacity: DEFAULT_RING_CAPACITY,
                    durable_capacity: DEFAULT_DURABLE_CAPACITY,
                    agent_handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
                    park_after: None,
                    private_dir: None,
                },
                stop.clone(),
            ));
            let key = NodeKeypair::generate().unwrap();
            let private = "private-diagnostic-source-canary";
            let id = NodeId::random();
            let lifecycle = Lifecycle::new(
                NodeInfo {
                    id,
                    name: private.to_owned(),
                    hostname: private.to_owned(),
                    os: private.to_owned(),
                    arch: private.to_owned(),
                    version: private.to_owned(),
                    protocol: ProtocolRange::CURRENT,
                    started_at: DateTime::UNIX_EPOCH,
                    state: NodeState::Serving,
                    public_key: Some(*key.public()),
                    timezone: None,
                },
                stop,
                Arc::clone(&runtime),
            );
            let started = Instant::now();
            tokio::time::advance(OBSERVATION_GAP).await;
            let facts = observation(&lifecycle, started);
            assert_eq!(facts.id, id);
            assert_eq!(facts.state, NodeState::Serving);
            assert_eq!(facts.uptime, DurationMs::from_duration(OBSERVATION_GAP));
            assert_eq!(facts.home, DiagnosticHome::Redacted);
            let encoded = serde_json::to_string(&facts).unwrap();
            assert!(!encoded.contains(private));
            assert!(!encoded.contains(&key.public().to_string()));
            assert!(!format!("{facts:?}").contains(private));
            lifecycle.stop(StopWhen::WhenIdle);
            assert_eq!(observation(&lifecycle, started).state, NodeState::Draining);
            phase("empty runtime shuts down", runtime.shutdown()).await;
        })
        .await;
    }
}
