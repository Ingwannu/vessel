//! `session.resume` (protocol 10, ADR 0026): an ended structured session
//! started again, granted as a session the caller creates would be.
//!
//! [Decision Log]
//! - Purpose: the contract's `session.resume` and its review item 3: the
//!   session's agent tools get a new credential within the caller's grant
//!   (`Agents::reserve`, as `session.create` does: under the owner for the
//!   owner or a device, under the calling agent for an agent, whose live
//!   sessions it then counts), its `created_by` stays its creator's, and
//!   its agent asks questions as a new session of that creator would.
//! - Order, so a refusal starts nothing and leaves nothing: draining, then
//!   the claim (one start again per session), then what the stored row
//!   says (`Sessions::ended`), then its profile, then the grant and a
//!   routed profile's token; only then is a process started. A failure
//!   after the grant gives the places back, as `session.create` does.
//! - A routed session keeps its route (ADR 0015), as a move does: a
//!   profile whose route changed since is refused (`unsupported`) rather
//!   than run the conversation through another provider.

use vessel_proto::workspace::SessionPlace;
use vessel_proto::{SessionId, SessionInfo};

use super::{Connection, NodeService};
use crate::error::ServiceError;
use crate::sessions::again;

impl NodeService {
    /// Start the ended session `id` again for `connection`'s caller; its
    /// info once its agent is ready.
    /// `rehydrate`, a reload's (TRACKER 803): the stored log's user
    /// prompts, which the agent's replay is lined up with instead of being
    /// dropped.
    pub(super) async fn start_again(
        &self,
        connection: &Connection,
        id: SessionId,
        rehydrate: Option<vessel_session::UserPrompts>,
    ) -> Result<SessionInfo, ServiceError> {
        if self.lifecycle.is_draining() {
            return Err(ServiceError::Draining);
        }
        let claim = self.sessions.claim(id)?;
        let ended = self.sessions.ended(id).await?;
        if let Some(parent) = ended.info.spec.side_question_of {
            crate::side_questions::person(connection.principal())?;
            let _parent = self.sessions.get(parent).await?;
        }
        self.refuse_installing(&ended.info.spec.harness)?;
        // Not into a worktree a removal (or setup) holds: the removal may
        // take the folder from under the agent. Held, once checked, until
        // the session is live (ADR 0030), so no removal starts meanwhile.
        let _starting = match ended.info.spec.place.as_deref() {
            Some(SessionPlace::Worktree { worktree }) => Some(
                self.workspaces
                    .start_in(*worktree)
                    .ok_or(ServiceError::WorktreeBusy(*worktree))?,
            ),
            Some(SessionPlace::Project { .. }) | None => None,
        };
        let mut launch = self.launcher.again(&self.profiles, &ended.info)?;
        launch.rehydrate = rehydrate;
        // The schedules its agent made in its last run go first: a run
        // started again is a new grant, and a heartbeat does not outlive
        // the run that set it (ADR 0033, "Integration 3 (node)").
        // Its only failure is a scheduler that has stopped: the node is.
        self.schedules
            .starting_again(id)
            .await
            .map_err(|_stopped| ServiceError::Draining)?;
        let secret = self
            .agents
            .reserve(id, &connection.principal, self.launcher.has_tools())?;
        let granted = async {
            if let Some(secret) = &secret {
                self.launcher
                    .attach_tools(&mut launch.mcp_servers, id, secret)?;
            }
            let launched = self.models.launch(launch.profile.as_ref(), id)?;
            let route = launched.as_ref().map(|(model, _proxy)| &model.route);
            let recorded = ended.info.model.as_ref().map(|model| &model.route);
            if route != recorded {
                return Err(ServiceError::MoveRoute {
                    from: recorded.cloned(),
                    to: route.cloned(),
                });
            }
            launch.model_proxy = launched.map(|(_model, proxy)| proxy);
            Ok(())
        };
        let started = match granted.await {
            Ok(()) => self.sessions.start_again(ended, launch, claim).await,
            Err(error) => Err(error),
        };
        let handle = match started {
            Ok(handle) => handle,
            Err(error) => {
                self.agents.forget(id);
                self.models.retire(id);
                return Err(error);
            }
        };
        self.watch_session(&handle, &connection.principal);
        again::ready(&handle).await
    }
}
