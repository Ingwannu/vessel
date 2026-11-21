//! A reload's part in `Sessions` (TRACKER 803, ADR 0110): the stored log's
//! user prompts before a run, and a live session's reload through the
//! runtime. What a reload does to the agent is the session runtime's
//! (`vessel_session`'s `reload`).

use std::sync::Arc;

use vessel_proto::{EventSeq, SessionId, SessionInfo};
use vessel_session::{Reload, SessionError, SessionHandle, UserPrompts};

use super::{Found, Sessions};
use crate::agents::ToolMove;
use crate::error::ServiceError;

/// Records read from the store at a time while folding.
const PAGE: u32 = 1000;

impl Sessions {
    /// The user prompts of `id`'s stored records before `until` (where a
    /// run starts, so every record before it is stored). Unknown when the
    /// first records were pruned: a reload then adds nothing.
    pub(crate) async fn prompts_before(
        &self,
        id: SessionId,
        until: EventSeq,
    ) -> Result<UserPrompts, ServiceError> {
        let mut prompts = UserPrompts::new();
        let mut from = EventSeq::ZERO;
        while from < until {
            let page = self.store.read_events(id, from, PAGE).await?;
            if page.records.first().is_some_and(|record| record.seq > from) {
                return Ok(UserPrompts::unknown());
            }
            for record in page.records.iter().take_while(|record| record.seq < until) {
                prompts.observe(&record.event);
            }
            if page.at_end || page.next <= from {
                break;
            }
            from = page.next;
        }
        Ok(prompts)
    }

    /// Reload the live session `id` with what `plan` makes of it; an ended
    /// one is `Ended` (the service starts it again instead).
    pub(crate) async fn reload(
        &self,
        id: SessionId,
        plan: impl AsyncFnOnce(&SessionHandle) -> Result<(Reload, Option<ToolMove>), ServiceError>,
    ) -> Result<SessionInfo, ServiceError> {
        match self.find(id).await? {
            Found::Live { handle, .. } if handle.has_ended() => Err(ServiceError::Ended(id)),
            Found::Live { handle, .. } => {
                let (reload, tools) = plan(&handle).await?;
                let runtime = Arc::clone(&self.runtime);
                self.finish_replacement(id, tools, async move {
                    runtime.reload(id, reload).await
                })
                .await
            }
            Found::Stored(_) => Err(ServiceError::Ended(id)),
        }
    }

    /// Once the replacement has a launch plan, its credential owner waits
    /// for the actor even if the RPC is cancelled. The preparation stays
    /// cancellable before the plan is accepted.
    pub(super) async fn finish_replacement(
        &self,
        id: SessionId,
        tools: Option<ToolMove>,
        run: impl Future<Output = Result<SessionInfo, SessionError>> + Send + 'static,
    ) -> Result<SessionInfo, ServiceError> {
        let models = Arc::clone(&self.models);
        self.tasks
            .spawn(async move {
                let result = run.await;
                models.end_move(id, result.is_ok());
                if let Some(tools) = tools {
                    tools.finish(result.is_ok());
                }
                result.map_err(ServiceError::from)
            })
            .await
            .map_err(ServiceError::Task)?
    }
}
