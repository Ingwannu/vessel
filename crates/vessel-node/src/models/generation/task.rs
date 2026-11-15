//! Own one original session handle until the first-input task finishes or loses title ownership.

use super::super::Models;
use super::{client::Client, driver, preparation::Context, source};
use crate::settings::Settings;
use std::{path::PathBuf, sync::Arc};
use tokio_util::{sync::CancellationToken, task::TaskTrackerToken};
use vessel_session::SessionHandle;
use vessel_workspace::Git;

pub(super) struct Task {
    pub(super) handle: SessionHandle,
    pub(super) models: Arc<Models>,
    pub(super) settings: Arc<Settings>,
    pub(super) cwd: PathBuf,
    pub(super) git: Option<Git>,
    pub(super) stop: CancellationToken,
    pub(super) work: TaskTrackerToken,
}
impl Task {
    pub(super) async fn run(&self, client: &Client) {
        if self.stop.is_cancelled() {
            return;
        }
        tokio::select! {
            biased;
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
        let metadata = self.settings.metadata().await;
        let expected = seed.expected.clone();
        tokio::select! {
            biased;
            () = source::lost(&self.handle, &expected) => {
                tracing::debug!(session = %self.handle.id(), "title generation cancelled after losing its first source");
            },
            result = driver::generate(&self.models, client, &self.handle, &metadata,
                seed, Context { cwd: self.cwd.clone(), git: self.git.clone(), tracked: self.work.clone() }) => {
                match result {
                    Ok(outcome) => tracing::debug!(session = %self.handle.id(), ?outcome, "title generation finished"),
                    Err(error) => tracing::warn!(session = %self.handle.id(), %error, "title generation kept the provisional title"),
                }
            }
        }
    }
}
