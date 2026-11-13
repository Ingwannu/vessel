//! Protocol 11's saved-prompt calls (milestone 11, track D, ADR 0033), for
//! the CLI and the agent tools. Each is [`NodeClient::call`] with its
//! request.

use vessel_proto::automation::{
    CreatePrompt, DeletePrompt, ListPrompts, SavedPrompt, UpdatePrompt,
};
use vessel_proto::{ProjectId, SavedPromptId};
use vessel_rpc::CallError;

use crate::NodeClient;

impl NodeClient {
    /// `prompt.list`: every saved prompt, or `project`'s and the
    /// machine's, by name.
    ///
    /// # Errors
    /// `not_found` for an unknown project; `NotInProtocol` before protocol
    /// 11; otherwise see [`CallError`].
    pub async fn saved_prompts(
        &self,
        project: Option<ProjectId>,
    ) -> Result<Vec<SavedPrompt>, CallError> {
        self.call(ListPrompts { project })
            .await
            .map(|list| list.prompts)
    }

    /// `prompt.create`.
    ///
    /// # Errors
    /// `conflict` (a name its scope has, the node's bound), `not_found`
    /// (the project, the session's place), `invalid_request` (a terminal
    /// or resuming spec, a place in another project), `forbidden`
    /// (agents); otherwise see [`CallError`].
    pub async fn create_prompt(&self, request: CreatePrompt) -> Result<SavedPrompt, CallError> {
        self.call(request).await
    }

    /// `prompt.update`: its project stays.
    ///
    /// # Errors
    /// `conflict` for another revision; as
    /// [`create_prompt`](Self::create_prompt).
    pub async fn update_prompt(&self, request: UpdatePrompt) -> Result<SavedPrompt, CallError> {
        self.call(request).await
    }

    /// `prompt.delete`.
    ///
    /// # Errors
    /// `not_found`, `forbidden` (agents).
    pub async fn delete_prompt(&self, prompt: SavedPromptId) -> Result<(), CallError> {
        self.call(DeletePrompt { prompt }).await.map(|_ack| ())
    }
}
