//! Side context waits for persistence, then reads an independent plan and
//! a bounded newest-first tail. Dedicated input bypasses the generic side
//! refusal only after the service has checked the caller and built context.

use vessel_proto::{Delivered, SessionId};
use vessel_store::SideContext;

use super::{Found, Sessions};
use crate::error::ServiceError;
use crate::persist;
use crate::side_questions::SideError;

impl Sessions {
    pub(crate) async fn side_context(
        &self,
        parent: SessionId,
    ) -> Result<SideContext, ServiceError> {
        let through = match self.find(parent).await? {
            Found::Live {
                handle,
                mut durability,
            } => {
                let through = handle.info().next_seq;
                persist::wait_until(parent, &mut durability, through).await?;
                through
            }
            Found::Stored(info) => info.next_seq,
        };
        Ok(self.store.side_context(parent, through).await?)
    }

    pub(crate) async fn side_prompt(
        &self,
        side: SessionId,
        question: vessel_proto::automation::PromptText,
        context: vessel_agent::PromptContext,
    ) -> Result<Delivered, ServiceError> {
        match self.find(side).await? {
            Found::Live { handle, .. } => {
                if handle.info().spec.side_question_of.is_none() {
                    return Err(SideError::NotSide.into());
                }
                Ok(handle.side_question(question, context).await?)
            }
            Found::Stored(_) => Err(ServiceError::Ended(side)),
        }
    }
}

impl Sessions {
    /// Avoid a transient reset refusal between process exit and store
    /// hand-over. Subscribe before checking so the end cannot be missed.
    pub(crate) async fn wait_side_stored(&self, side: SessionId) -> Result<(), ServiceError> {
        let mut ended = self.ends();
        let wait = async {
            loop {
                if matches!(self.find(side).await?, Found::Stored(_)) {
                    return Ok(());
                }
                match ended.recv().await {
                    Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        return Err(ServiceError::Draining);
                    }
                }
            }
        };
        tokio::time::timeout(std::time::Duration::from_secs(10), wait)
            .await
            .map_err(|_elapsed| ServiceError::LogWriting(side))?
    }
}
