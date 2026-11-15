//! Runtime construction keeps node-owned persistence adapters outside session state.
use crate::config::Config;
use std::{path::PathBuf, sync::Arc};
use tokio_util::sync::CancellationToken;
use vessel_session::{AttachmentLoader, RuntimeConfig, SessionRuntime};
use vessel_store::Store;

/// The session runtime, whose structured sessions load their prompts'
/// attachments through `loader` and whose direct drivers write a launch's
/// private files only in `private_dir`.
pub(super) fn runtime(
    config: &Config,
    loader: AttachmentLoader,
    private_dir: PathBuf,
    store: &Store,
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
        .with_attachments(loader)
        .with_questions(Arc::new(crate::question_bridge::Questions::new(
            store.clone(),
        ))),
    )
}
