//! Model routes and profiles (ADRs 0011, 0015), and the ownership rule
//! agents' `operate` calls are held to (ADR 0013). Routing only.

use std::sync::Arc;

use vessel_proto::{
    ListModels, ListProfiles, ModelRoute, MoveProfile, ProfileInfo, ReloadProfiles, SessionId,
    SessionInfo, WireError,
};

use crate::error::ServiceError;
use crate::principal::Principal;
use crate::profiles::moving;

use super::{Connection, NodeService, wire};

/// Model routes (protocol 7, milestone 5, ADR 0015).
impl NodeService {
    /// `model.list`: the routes this node offers; names, providers, APIs
    /// and model ids, never a URL or a secret name.
    #[expect(
        clippy::unused_async,
        reason = "every call has the same async shape, so the guard's rows are uniform"
    )]
    pub(crate) async fn list_models(
        &self,
        _: &Connection,
        _: ListModels,
    ) -> Result<Vec<ModelRoute>, WireError> {
        Ok(self.models.list())
    }
}

/// Profiles (ADR 0011, Track H).
impl NodeService {
    /// `profile.list`: names, labels, harnesses and variable names; never a
    /// value.
    #[expect(
        clippy::unused_async,
        reason = "every call has the same async shape, so the guard's rows are uniform"
    )]
    pub(crate) async fn list_profiles(
        &self,
        _: &Connection,
        request: ListProfiles,
    ) -> Result<Vec<ProfileInfo>, WireError> {
        Ok(self.profiles.list(request.harness.as_ref()))
    }

    /// `profile.reload`: read `profiles.toml` again; a file that cannot be
    /// used leaves the profiles in effect and says where it is wrong.
    pub(crate) async fn reload_profiles(
        &self,
        _: &Connection,
        _: ReloadProfiles,
    ) -> Result<Vec<ProfileInfo>, WireError> {
        let profiles = Arc::clone(&self.profiles);
        let models = Arc::clone(&self.models);
        tokio::task::spawn_blocking(move || {
            // `models.toml` goes with the profiles that name its routes;
            // either file that cannot be used leaves its own in effect.
            let reloaded = profiles.reload().map_err(ServiceError::Profiles)?;
            models.reload().map_err(ServiceError::Models)?;
            models.check_profiles(&reloaded);
            Ok(reloaded)
        })
        .await
        .map_err(|error| wire(&ServiceError::Task(error)))?
        .map_err(|error: ServiceError| wire(&error))
    }

    /// `session.move_profile` (H2): a live structured session resumes its
    /// conversation under another profile (`profiles::moving`, then the
    /// session runtime).
    pub(crate) async fn move_profile(
        &self,
        _: &Connection,
        request: MoveProfile,
    ) -> Result<SessionInfo, WireError> {
        let id = request.session;
        let profiles = &self.profiles;
        let launcher = &self.launcher;
        let lookup = &self.lookup;
        let agents = &self.agents;
        let models = &self.models;
        let moved = self
            .sessions
            .move_profile(id, async |info: &SessionInfo| {
                let mut plan = moving::plan(profiles, launcher, lookup, info, &request).await?;
                // The restarted agent's `vessel mcp` gets a credential of
                // its own (`agents`, Decision Log), and a routed session's
                // agent a model token of its own, on the same route.
                let route = info.model.as_ref().map(|model| &model.route);
                plan.model_proxy = models.begin_move(id, route, &plan.profile)?;
                if let Some(secret) = agents.begin_move(id)? {
                    launcher.attach_tools(&mut plan.mcp_servers, id, &secret)?;
                }
                Ok(plan)
            })
            .await;
        self.agents.end_move(id, moved.is_ok());
        self.models.end_move(id, moved.is_ok());
        moved
            .map(|info| self.with_workspace(info))
            .map_err(|error| wire(&error))
    }

    /// For an agent, an `operate` call on `session` needs the session to
    /// have been started by the agent's session or a session it started,
    /// and so on (ADR 0013); anyone else passes. The agent may be another
    /// node's whose call was delivered here (protocol 6): the chain is
    /// followed through this node's sessions until it reaches the agent's
    /// `(node, session)`.
    pub(crate) async fn check_owner(
        &self,
        principal: &Principal,
        session: SessionId,
    ) -> Result<(), WireError> {
        let Some(agent) = principal.agent() else {
            return Ok(());
        };
        // This node's agent is done once its session's end is visible,
        // before `retire` closes its connection (as in `Agents::admit`).
        // Another node's agent was checked by its own node when the call
        // was passed on (`Agents::depth`).
        if matches!(principal, Principal::Agent { .. }) && !self.agents.is_live(agent.1) {
            return Err(wire(&ServiceError::AgentGone(agent.1)));
        }
        self.sessions
            .check_parentage(
                self.lifecycle.info().id,
                vessel_proto::SessionRef {
                    node: agent.0,
                    session: agent.1,
                },
                session,
            )
            .await
            .map_err(|error| wire(&error))
    }
}
