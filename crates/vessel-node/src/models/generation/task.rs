//! Own one original session handle until the first-input task finishes or loses title ownership.

use super::super::Models;
use super::{client::Client, driver, source};
use crate::settings::Settings;
use std::{path::PathBuf, sync::Arc};
use tokio_util::sync::CancellationToken;
use vessel_session::SessionHandle;
use vessel_workspace::Git;

pub(super) struct Task {
    pub(super) handle: SessionHandle,
    pub(super) models: Arc<Models>,
    pub(super) settings: Arc<Settings>,
    pub(super) cwd: PathBuf,
    pub(super) git: Option<Git>,
    pub(super) stop: CancellationToken,
}
impl Task {
    pub(super) async fn run(&self, client: &Client) {
        tokio::select! {
            () = self.stop.cancelled() => {},
            () = self.work(client) => {},
        }
    }

    async fn work(&self, client: &Client) {
        let seed = match source::first(&self.handle).await {
            Ok(seed) => seed,
            Err(error) => {
                tracing::debug!(session = %self.handle.id(), %error, "title source not used");
                return;
            }
        };
        let settings = self.settings.current().await;
        let expected = seed.expected.clone();
        tokio::select! {
            () = source::lost(&self.handle, &expected) => {},
            result = driver::generate(&self.models, client, &self.handle, &settings.metadata,
                seed, self.cwd.clone(), self.git.clone()) => {
                match result {
                    Ok(outcome) => tracing::debug!(session = %self.handle.id(), ?outcome, "title generation finished"),
                    Err(error) => tracing::warn!(session = %self.handle.id(), %error, "title generation kept the provisional title"),
                }
            }
        }
    }
}
