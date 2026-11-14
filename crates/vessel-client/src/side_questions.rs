//! Typed side-question calls (protocol 19, ADR 0045). They do not inject
//! anything into the original conversation.

use vessel_proto::automation::PromptText;
use vessel_proto::side_questions::{OpenSideQuestion, PromptSideQuestion, ResetSideQuestion};
use vessel_proto::{Delivered, SessionId, SessionInfo};

use crate::{CallError, NodeClient};

impl NodeClient {
    /// Open or reuse the parent's newest unarchived side conversation.
    /// An ended side is returned unchanged; resuming it is explicit.
    ///
    /// # Errors
    /// Missing parent; terminal/nested parent; insufficient read/operate
    /// scopes or an agent caller; ordinary launch failures; older node.
    pub async fn open_side_question(&self, parent: SessionId) -> Result<SessionInfo, CallError> {
        self.call(OpenSideQuestion { session: parent }).await
    }

    /// Ask a side question with a bounded snapshot of its parent's latest
    /// state. Refuses a busy or ended side; it never queues a question.
    ///
    /// # Errors
    /// As opening; a non-side id; busy/ended side; parent removed.
    pub async fn prompt_side_question(
        &self,
        side: SessionId,
        question: PromptText,
    ) -> Result<Delivered, CallError> {
        self.call(PromptSideQuestion {
            session: side,
            question,
        })
        .await
    }

    /// Stop and delete the expected side, then create its replacement.
    /// Creation failure after deletion leaves the old side deleted.
    ///
    /// # Errors
    /// As opening; stale expected id; stop/delete failure. Opening again
    /// resolves a failed replacement without deleting a newer side.
    pub async fn reset_side_question(
        &self,
        parent: SessionId,
        side: SessionId,
    ) -> Result<SessionInfo, CallError> {
        self.call(ResetSideQuestion {
            session: parent,
            side,
        })
        .await
    }
}
