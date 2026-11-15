//! What reaches a live session's conversation: input, prompts delivered
//! as asked and queue changes (protocol 10), each prompt's attachments
//! replaced by the node's own records first; and the conversation as it
//! stands, live or stored.

use vessel_proto::conversation::QueueChanged;
use vessel_proto::{
    ContentBlock, Conversation, Creator, Delivered, Delivery, EventSeq, QueueChange, SessionId,
    SessionInput, SessionMode,
};
use vessel_session::{SessionError, SessionHandle};

use super::{Found, Sessions};
use crate::error::ServiceError;
use crate::persist;

const SNAPSHOT_COMMIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

impl Sessions {
    pub(crate) async fn send(
        &self,
        id: SessionId,
        input: SessionInput,
    ) -> Result<(), ServiceError> {
        match self.find(id).await? {
            Found::Live { handle, .. } => {
                let input = match input {
                    SessionInput::Prompt { content } => SessionInput::Prompt {
                        content: self.attached(&handle, content).await?,
                    },
                    other @ (SessionInput::Terminal { .. }
                    | SessionInput::CancelTurn { .. }
                    | SessionInput::AnswerPermission { .. }
                    | SessionInput::AnswerQuestion { .. }) => other,
                };
                Ok(handle.send(input).await?)
            }
            Found::Stored(_) => Err(ServiceError::Ended(id)),
        }
    }

    /// Deliver a prompt to a live structured session (protocol 10).
    pub(crate) async fn prompt(
        &self,
        id: SessionId,
        content: Vec<ContentBlock>,
        delivery: Delivery,
        by: Creator,
    ) -> Result<Delivered, ServiceError> {
        match self.find(id).await? {
            Found::Live { handle, .. } => {
                let content = self.attached(&handle, content).await?;
                Ok(handle.prompt(content, delivery, by).await?)
            }
            Found::Stored(_) => Err(ServiceError::Ended(id)),
        }
    }

    /// As [`prompt`](Self::prompt), and which turn it started or joined
    /// (protocol 11's `notify`; `SessionHandle::prompt_tracked`).
    pub(crate) async fn prompt_tracked(
        &self,
        id: SessionId,
        content: Vec<ContentBlock>,
        delivery: Delivery,
        by: Creator,
    ) -> Result<(Delivered, tokio::sync::oneshot::Receiver<EventSeq>), ServiceError> {
        match self.find(id).await? {
            Found::Live { handle, .. } => {
                let content = self.attached(&handle, content).await?;
                Ok(handle.prompt_tracked(content, delivery, by).await?)
            }
            Found::Stored(_) => Err(ServiceError::Ended(id)),
        }
    }

    /// Change a live structured session's queue (protocol 10). An ended
    /// session's queue comes back with `session.resume`.
    pub(crate) async fn change_queue(
        &self,
        id: SessionId,
        change: QueueChange,
    ) -> Result<QueueChanged, ServiceError> {
        match self.find(id).await? {
            Found::Live { handle, .. } => {
                if handle.info().spec.side_question_of.is_some() {
                    return Err(crate::side_questions::SideError::PromptOnly.into());
                }
                let change = match change {
                    QueueChange::Edit {
                        entry,
                        revision,
                        content,
                    } => QueueChange::Edit {
                        entry,
                        revision,
                        content: self.attached(&handle, content).await?,
                    },
                    other @ (QueueChange::Move { .. }
                    | QueueChange::Remove { .. }
                    | QueueChange::SendNow { .. }
                    | QueueChange::Hold
                    | QueueChange::Resume
                    | QueueChange::Clear) => other,
                };
                Ok(handle.change_queue(change).await?)
            }
            Found::Stored(_) => Err(ServiceError::Ended(id)),
        }
    }

    /// A prompt for `handle`'s session with each attachment it names
    /// replaced by the node's record (protocol 10, ADR 0025), so the
    /// session, its queue and its log see only what the node kept. A
    /// terminal session's prompt is left for the handle to refuse.
    async fn attached(
        &self,
        handle: &SessionHandle,
        content: Vec<ContentBlock>,
    ) -> Result<Vec<ContentBlock>, ServiceError> {
        if handle.info().spec.side_question_of.is_some() {
            return Err(crate::side_questions::SideError::PromptOnly.into());
        }
        let named = content
            .iter()
            .any(|block| matches!(block, ContentBlock::Attachment { .. }));
        if !named || matches!(handle.info().spec.mode, SessionMode::Terminal { .. }) {
            return Ok(content);
        }
        crate::attachments::load::resolve(&self.store, handle.id(), content).await
    }

    /// A structured session's conversation as it stands (protocol 10): a
    /// live one's from its runtime, an ended one's from the store.
    pub(crate) async fn conversation(&self, id: SessionId) -> Result<Conversation, ServiceError> {
        let mut conversation = match self.find(id).await? {
            Found::Live {
                handle,
                mut durability,
            } => {
                let snapshot = handle.conversation()?;
                if let Some(through) = snapshot.through {
                    tokio::time::timeout(
                        SNAPSHOT_COMMIT_TIMEOUT,
                        persist::wait_until(id, &mut durability, through),
                    )
                    .await
                    .map_err(|_elapsed| persist::Unpersisted(id))??;
                }
                Ok(snapshot)
            }
            Found::Stored(info) => match info.spec.mode {
                SessionMode::Terminal { .. } => {
                    Err(SessionError::Terminal("a conversation").into())
                }
                SessionMode::Structured { .. } => self
                    .store
                    .conversation(id)
                    .await?
                    .ok_or(ServiceError::NotFound(id)),
            },
        }?;
        conversation
            .questions
            .retain(|question| question.lifetime == vessel_proto::timeline::QuestionLifetime::Turn);
        conversation
            .questions
            .extend(self.store.pending_questions(id).await?);
        Ok(conversation)
    }
}

impl Sessions {
    pub(crate) async fn inbox_request(
        &self,
        id: SessionId,
        request: vessel_proto::inbox::InboxRequest,
    ) -> Result<Option<vessel_proto::inbox::InboxItem>, ServiceError> {
        let found = self.find(id).await?;
        if let vessel_proto::inbox::InboxRequest::Question { id: question_id } = request {
            if let Some(question) = self
                .store
                .pending_questions(id)
                .await?
                .into_iter()
                .find(|question| question.id == question_id)
            {
                return Ok(Some(vessel_proto::inbox::InboxItem::Question { question }));
            }
        }
        match found {
            Found::Live { handle, .. } => {
                let item = handle.inbox_request(request)?;
                if matches!(&item, Some(vessel_proto::inbox::InboxItem::Question { question }) if question.lifetime == vessel_proto::timeline::QuestionLifetime::Session)
                {
                    return Ok(None);
                }
                Ok(item)
            }
            Found::Stored(_) => Ok(None),
        }
    }
}
