//! Live bundle routing and selected probes through the existing runtime.

use super::{Found, Sessions};
use crate::error::ServiceError;
use std::path::PathBuf;
use vessel_harness::{ModelProxy, Profile};
use vessel_proto::{HarnessRef, SessionId, launch_profiles::LaunchChoices};

impl Sessions {
    pub(crate) async fn apply_options(
        &self,
        id: SessionId,
        choices: LaunchChoices,
    ) -> Result<vessel_session::ConfigurationOutcome, ServiceError> {
        match self.find(id).await? {
            Found::Live { handle, .. } => Ok(handle.apply_options(choices).await?),
            Found::Stored(_) => Err(ServiceError::Ended(id)),
        }
    }

    pub(crate) async fn probe_choices(
        &self,
        harness: HarnessRef,
        profile: Profile,
        proxy: Option<ModelProxy>,
        dir: PathBuf,
        choices: LaunchChoices,
    ) -> Result<vessel_session::Probed, ServiceError> {
        self.runtime
            .probe_choices(harness, Some(profile), proxy, dir, choices)
            .await
            .map_err(ServiceError::Session)
    }
}
