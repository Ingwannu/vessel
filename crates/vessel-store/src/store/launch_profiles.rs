//! The public async boundary for complete launch-profile snapshots and CAS.

use crate::writer::Write;
use crate::{LaunchProfilesChange, Store, StoreError};
use tokio::sync::oneshot;
use vessel_proto::launch_profiles::{
    LaunchProfileRevision, LaunchProfiles, LaunchProfilesSnapshot,
};

impl Store {
    /// Read the entire bounded ordered list and its revision atomically.
    ///
    /// # Errors
    /// Storage failure or corrupt/oversized data; never an empty fallback.
    pub async fn launch_profiles(&self) -> Result<LaunchProfilesSnapshot, StoreError> {
        self.read(|reader| reader.with(crate::launch_profiles::read))
            .await
    }

    /// Replace the complete list if its committed revision still matches.
    /// An acknowledgement lost after admission requires rereading, not replay.
    ///
    /// # Errors
    /// The write/savepoint/commit failed, or the writer has closed.
    pub async fn replace_launch_profiles(
        &self,
        expected: LaunchProfileRevision,
        profiles: LaunchProfiles,
    ) -> Result<LaunchProfilesChange, StoreError> {
        let (done, result) = oneshot::channel();
        self.send(Write::LaunchProfiles {
            expected,
            profiles,
            done,
        })
        .await?;
        result.await.unwrap_or(Err(StoreError::Closed))
    }
}
