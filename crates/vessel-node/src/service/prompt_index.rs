//! Metadata-only outline reads; the guard owns admission and the store owns work.

use vessel_proto::WireError;
use vessel_proto::prompt_index::{ListSessionPrompts, PromptIndexPage};

use super::{Connection, NodeService, wire};
use crate::error::ServiceError;

impl NodeService {
    /// Read the same retained history the caller can already read via session.read.
    /// Store admission stays with the blocking snapshot if the requester leaves.
    pub(crate) async fn list_session_prompts(
        &self,
        _: &Connection,
        request: ListSessionPrompts,
    ) -> Result<PromptIndexPage, WireError> {
        self.sessions
            .store()
            .prompt_index(request)
            .await
            .map_err(|error| wire(&ServiceError::Store(error)))
    }
}
