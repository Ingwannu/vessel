//! Normalize the prior daemon only after its saved recovery policy is known.
use crate::{config::Config, error::NodeError};
use vessel_store::{Opened, Store};
/// Open the event log, closing sessions a previous run left live.
pub(super) async fn open(config: &Config, enabled: bool) -> Result<Store, NodeError> {
    let Opened { store, interrupted } =
        Store::open_with_restart_policy(&config.data_dir, config.store, enabled)
            .await
            .map_err(NodeError::Store)?;
    if !interrupted.is_empty() {
        tracing::info!(
            sessions = interrupted.len(),
            "sessions from the previous run are now interrupted"
        );
    }
    Ok(store)
}
