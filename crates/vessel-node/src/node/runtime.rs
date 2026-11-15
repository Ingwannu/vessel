//! Bind the session runtime to node-owned configuration and attachment loading.
//! It must not start transports, grant authority or open settings files.
use crate::config::Config;
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use vessel_session::{AttachmentLoader, RuntimeConfig, SessionRuntime};

/// The session runtime, whose structured sessions load their prompts'
/// attachments through `loader` and whose direct drivers write a launch's
/// private files only in `private_dir`.
pub(super) fn open(
    config: &Config,
    loader: AttachmentLoader,
    private_dir: PathBuf,
) -> Arc<SessionRuntime> {
    Arc::new(
        SessionRuntime::new(
            RuntimeConfig {
                harness_lookup: config.harness_lookup.clone(),
                harness_env: config.harness_env.clone(),
                ring_capacity: config.ring_capacity,
                durable_capacity: config.durable_capacity,
                agent_handshake_timeout: config.agent_handshake_timeout,
                park_after: config.park_after,
                private_dir: Some(private_dir),
            },
            CancellationToken::new(),
        )
        .with_attachments(loader),
    )
}
