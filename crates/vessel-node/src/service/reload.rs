//! `session.reload` (TRACKER 803, ADR 0110): Paseo's "Reload agent"
//! (`refresh_agent_request`, server `session.ts`).
//!
//! [Decision Log]
//! - Purpose: a person restarts a session's agent so it picks up skills,
//!   MCP servers and its login, and sees what the agent's own history has
//!   beyond the log (turns made outside Vessel).
//! - Order, as Paseo's handler: refuse an agent (Paseo has no tool for it;
//!   a person reloads), unarchive (`unarchiveAgentState`, kept even if the
//!   reload then fails, as there), then a live session is reloaded in place
//!   and an ended one is started again (Paseo's `ensureAgentLoaded` for a
//!   closed agent), both with the log's prompts so the replay is lined up
//!   rather than dropped.
//! - The live launch is a start again's (`Launcher::again`: the current
//!   profile, the fleet's MCP servers as they are now, notices) with a new
//!   `vessel` credential and a routed profile's new token, minted as a
//!   move mints them (`begin_move`/`end_move`): the old ones stay valid
//!   until the reload ends and are dropped only if it succeeded.
//! - Cost: the store's records before the run are read and folded on each
//!   reload (`Sessions::prompts_before`); a session started in this run has
//!   none to read.

use vessel_proto::reload::ReloadSession;
use vessel_proto::{Mark, SessionId, SessionInfo, WireError};
use vessel_session::{NotReloadable, Reload, SessionError, SessionHandle};

use super::{Connection, NodeService, wire};
use crate::error::ServiceError;

impl NodeService {
    /// `session.reload`: the session's agent started again on its
    /// conversation, answered once it is ready.
    pub(crate) async fn reload_session(
        &self,
        connection: &Connection,
        request: ReloadSession,
    ) -> Result<SessionInfo, WireError> {
        self.reload(connection, request.session)
            .await
            .map(|info| self.with_workspace(info))
            .map_err(|error| wire(&error))
    }

    async fn reload(
        &self,
        connection: &Connection,
        id: SessionId,
    ) -> Result<SessionInfo, ServiceError> {
        if connection.principal().agent().is_some() {
            return Err(SessionError::from(NotReloadable::Agent).into());
        }
        if self.lifecycle.is_draining() {
            return Err(ServiceError::Draining);
        }
        let info = self.sessions.get(id).await?;
        if info.marks.archived_at.is_some() {
            let _unarchived = self.sessions.mark(id, Mark::Unarchive).await?;
        }
        let reloaded = self
            .sessions
            .reload(id, async |handle: &SessionHandle| {
                self.plan(id, handle).await
            })
            .await;
        self.agents.end_move(id, reloaded.is_ok());
        self.models.end_move(id, reloaded.is_ok());
        match reloaded {
            // Ended, or ending while this call ran: started again once its
            // log is all in the store (`Sessions::ended` waits for the
            // hand-over), so the prompts read from it are all of them.
            Err(ServiceError::Ended(_) | ServiceError::Session(SessionError::Ended)) => {
                let info = self.sessions.get(id).await?;
                let _conversation = vessel_session::resumable(&info)
                    .map_err(|refusal| SessionError::from(NotReloadable::from(refusal)))?;
                let ended = self.sessions.ended(id).await?;
                let before = self
                    .sessions
                    .prompts_before(id, ended.info.next_seq)
                    .await?;
                self.start_again(connection, id, Some(before)).await
            }
            other => other,
        }
    }

    /// A live session's reload: its launch as a start again's, with a new
    /// credential and token, and its log's prompts before this run.
    async fn plan(&self, id: SessionId, handle: &SessionHandle) -> Result<Reload, ServiceError> {
        let info = handle.info();
        let before = self.sessions.prompts_before(id, handle.run_start()).await?;
        let mut launch = self.launcher.again(&self.profiles, &info)?;
        let profile = match &launch.profile {
            Some(profile) => profile.clone(),
            None => vessel_harness::Profile::default_of(&info.spec.harness)
                .map_err(|error| ServiceError::Session(SessionError::Launch(error)))?,
        };
        let route = info.model.as_ref().map(|model| &model.route);
        launch.model_proxy = self.models.begin_move(id, route, &profile)?;
        if let Some(secret) = self.agents.begin_move(id)? {
            self.launcher
                .attach_tools(&mut launch.mcp_servers, id, &secret)?;
        }
        Ok(Reload { launch, before })
    }
}
