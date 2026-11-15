//! Eligibility uses real session/account names, never client provider defaults.

use super::{Connection, NodeService, wire};
use crate::{error::ServiceError, launch_profiles::Lane};
use std::sync::Arc;
use vessel_proto::launch_profiles::{
    ApplicableLaunchProfiles, GetApplicableLaunchProfiles, LaunchProfiles,
};
use vessel_proto::{SessionMode, WireError};

impl NodeService {
    pub(crate) async fn applicable_launch_profiles(
        &self,
        _: &Connection,
        request: GetApplicableLaunchProfiles,
    ) -> Result<ApplicableLaunchProfiles, WireError> {
        let sessions = Arc::clone(&self.sessions);
        let profiles = Arc::clone(&self.profiles);
        self.launch_profiles
            .run(Lane::Metadata, move || async move {
                let info = sessions.get(request.session).await?;
                if !info.status.is_live() {
                    return Err(ServiceError::Ended(request.session));
                }
                match info.spec.mode {
                    SessionMode::Terminal { .. } => {
                        return Err(ServiceError::Session(
                            vessel_session::SessionError::NoOptions,
                        ));
                    }
                    SessionMode::Structured { .. } => {}
                }
                let accounts = profiles.current();
                let account = match info.profile.as_ref() {
                    Some(name) => name.clone(),
                    None => accounts
                        .default_of(&info.spec.harness)
                        .ok_or(ServiceError::LaunchHarnessGone)?
                        .name()
                        .clone(),
                };
                let stored = sessions
                    .store()
                    .launch_profiles()
                    .await
                    .map_err(ServiceError::Store)?;
                let eligible = stored
                    .profiles()
                    .as_slice()
                    .iter()
                    .filter(|profile| {
                        let launch = profile.launch();
                        if *launch.harness() != info.spec.harness {
                            return false;
                        }
                        let resolved = match launch.account_profile() {
                            Some(name) => accounts.get(name).cloned(),
                            None => accounts.default_of(launch.harness()),
                        };
                        resolved.is_some_and(|resolved| {
                            resolved.harness() == info.spec.harness && *resolved.name() == account
                        })
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let eligible =
                    LaunchProfiles::try_from(eligible).map_err(ServiceError::LaunchValues)?;
                Ok(ApplicableLaunchProfiles::new(stored.revision(), eligible))
            })
            .await
            .map_err(|error| wire(&error))
    }
}
