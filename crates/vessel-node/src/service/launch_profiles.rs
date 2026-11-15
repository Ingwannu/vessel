//! Launch metadata/CAS and live bundles, sharing existing storage and authority.

mod applicable;
mod options;

use super::{Connection, NodeService, wire};
use crate::error::ServiceError;
use crate::launch_profiles::Lane;
use crate::principal::Principal;
use std::sync::Arc;
use vessel_proto::WireError;
use vessel_proto::launch_profiles::{
    ApplySessionOptions, ApplySessionOptionsResult, LaunchConfigurationFailure,
    LaunchProfilesSnapshot, ListLaunchProfiles, ReplaceLaunchProfiles,
};
use vessel_store::LaunchProfilesChange;

impl NodeService {
    pub(crate) async fn list_launch_profiles(
        &self,
        _: &Connection,
        _: ListLaunchProfiles,
    ) -> Result<LaunchProfilesSnapshot, WireError> {
        let store = self.sessions.store().clone();
        self.launch_profiles
            .run(Lane::Metadata, move || async move {
                store.launch_profiles().await.map_err(ServiceError::Store)
            })
            .await
            .map_err(|error| wire(&error))
    }

    pub(crate) async fn replace_launch_profiles(
        &self,
        connection: &Connection,
        request: ReplaceLaunchProfiles,
    ) -> Result<LaunchProfilesSnapshot, WireError> {
        human(connection.principal()).map_err(|error| wire(&error))?;
        let store = self.sessions.store().clone();
        self.launch_profiles
            .run(Lane::Metadata, move || async move {
                match store
                    .replace_launch_profiles(request.expected, request.profiles)
                    .await
                    .map_err(ServiceError::Store)?
                {
                    LaunchProfilesChange::Unchanged(snapshot)
                    | LaunchProfilesChange::Replaced(snapshot) => Ok(snapshot),
                    LaunchProfilesChange::Stale { current } => {
                        Err(ServiceError::LaunchStale(current))
                    }
                    LaunchProfilesChange::Exhausted => Err(ServiceError::LaunchExhausted),
                }
            })
            .await
            .map_err(|error| wire(&error))
    }

    pub(crate) async fn apply_session_options(
        &self,
        _: &Connection,
        request: ApplySessionOptions,
    ) -> Result<ApplySessionOptionsResult, WireError> {
        let sessions = Arc::clone(&self.sessions);
        self.launch_profiles
            .run(Lane::Apply, move || async move {
                let done = sessions
                    .apply_options(request.session, request.options)
                    .await?;
                let (applied, notice, failure) = done.into_parts();
                let applied = applied.try_into().map_err(ServiceError::LaunchValues)?;
                let failure = failure.map(|failed| {
                    let (choice, error) = failed.into_parts();
                    let error = wire(&ServiceError::Session(vessel_session::SessionError::Input(
                        error,
                    )));
                    LaunchConfigurationFailure { choice, error }
                });
                ApplySessionOptionsResult::new(applied, notice, failure)
                    .map_err(ServiceError::LaunchValues)
            })
            .await
            .map_err(|error| wire(&error))
    }
}

fn human(principal: &Principal) -> Result<(), ServiceError> {
    if principal.agent().is_some() {
        Err(ServiceError::LaunchAgentOrigin)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
