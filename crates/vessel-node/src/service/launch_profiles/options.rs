//! A real selected probe, retaining one account/agent snapshot through publication.

use super::{Connection, NodeService, human, wire};
use crate::profiles::{NodeProfiles, launch::Launcher};
use crate::{error::ServiceError, launch_profiles::Lane};
use chrono::Utc;
use std::sync::Arc;
use vessel_harness::{Profile, Profiles};
use vessel_proto::launch_profiles::{
    GetLaunchProfileOptions, LaunchAccount, LaunchAccountKind, LaunchCatalogue,
    LaunchCatalogueOptions, LaunchSelection,
};
use vessel_proto::{SessionId, WireError};

impl NodeService {
    pub(crate) async fn launch_profile_options(
        &self,
        connection: &Connection,
        request: GetLaunchProfileOptions,
    ) -> Result<LaunchCatalogue, WireError> {
        human(connection.principal()).map_err(|error| wire(&error))?;
        self.refuse_installing(request.selection.harness())
            .map_err(|error| wire(&error))?;
        let profiles = Arc::clone(&self.profiles);
        let launcher = self.launcher.clone();
        let models = Arc::clone(&self.models);
        let sessions = Arc::clone(&self.sessions);
        let dir = self.probes.dir().clone();
        self.launch_profiles
            .run(Lane::Metadata, move || async move {
                let snapshot = profiles.current();
                let selection = request.selection;
                let profile = resolve(&snapshot, &profiles, &launcher, &selection)?;
                let kind = if profile.is_default() {
                    LaunchAccountKind::Default
                } else {
                    LaunchAccountKind::Named
                };
                let account = LaunchAccount::new(profile.name().clone(), kind);
                let id = SessionId::random();
                let proxy = models.launch(Some(&profile), id)?.map(|(_, proxy)| proxy);
                let result = sessions
                    .probe_choices(
                        selection.harness().clone(),
                        profile,
                        proxy,
                        dir,
                        selection.options().clone(),
                    )
                    .await;
                models.retire(id);
                // A failure from the retired definitions is stale too: it must
                // not replace a newer editor state with an old account error.
                if !Arc::ptr_eq(&snapshot, &profiles.current()) {
                    return Err(ServiceError::LaunchChanged);
                }
                let probed = result.map_err(structured)?;
                let options = LaunchCatalogueOptions::try_from(probed.options)
                    .map_err(ServiceError::LaunchValues)?;
                let result = LaunchCatalogue::new(selection, account, options, Utc::now())
                    .map_err(ServiceError::LaunchValues)?;
                Ok(result)
            })
            .await
            .map_err(|error| wire(&error))
    }
}

fn resolve(
    snapshot: &Profiles,
    owner: &NodeProfiles,
    launcher: &Launcher,
    selection: &LaunchSelection,
) -> Result<Profile, ServiceError> {
    let profile = match selection.account_profile() {
        Some(name) => snapshot
            .get(name)
            .cloned()
            .ok_or_else(|| ServiceError::UnknownProfile(name.clone()))?,
        None => snapshot
            .default_of(selection.harness())
            .ok_or(ServiceError::LaunchHarnessGone)?,
    };
    if profile.harness() != *selection.harness() {
        return Err(ServiceError::ProfileHarness {
            profile: profile.name().clone(),
            profile_harness: profile.harness(),
            harness: selection.harness().clone(),
        });
    }
    // Some(profile) deliberately bypasses added()'s default lookup: the
    // retained snapshot above is the only account/agent selection owner.
    launcher
        .added(owner, selection.harness(), Some(profile))?
        .ok_or(ServiceError::LaunchHarnessGone)
}

fn structured(error: ServiceError) -> ServiceError {
    if matches!(
        &error,
        ServiceError::Session(
            vessel_session::SessionError::Launch(
                vessel_harness::HarnessError::StructuredUnavailable { .. }
                    | vessel_harness::HarnessError::NoDirect { .. }
            ) | vessel_session::SessionError::StartAgent(vessel_agent::AgentError::NoDriver(_))
        )
    ) {
        ServiceError::LaunchStructuredUnavailable(Box::new(error))
    } else {
        error
    }
}
