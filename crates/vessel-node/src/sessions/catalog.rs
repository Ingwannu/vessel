//! Draft catalog probes share the session runtime's validated prompt-free launch.

use std::path::PathBuf;

use vessel_harness::{ModelProxy, Profile};
use vessel_proto::HarnessRef;
use vessel_session::{ProbeChoices, Probed};

use super::Sessions;
use crate::error::ServiceError;

impl Sessions {
    pub(crate) async fn probe_with_choices_owned<T: Send + Sync + 'static>(
        &self,
        harness: HarnessRef,
        profile: Profile,
        proxy: Option<ModelProxy>,
        dir: PathBuf,
        choices: ProbeChoices,
        ownership: T,
    ) -> Result<Probed, ServiceError> {
        Ok(self
            .runtime
            .probe_with_choices_owned(harness, Some(profile), proxy, dir, choices, ownership)
            .await?)
    }
}
