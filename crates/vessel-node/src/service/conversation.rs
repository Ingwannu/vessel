//! Protocol 10's calls (milestone 11's Track A, the conversation):
//! delivering a prompt, the queue, the conversation as it stands, starting
//! an ended session again, marks, deleting, and attachments. Routing only,
//! as the rest of the service.

use vessel_proto::conversation::{
    BeginUpload, ChangeQueue, DeleteSession, FinishUpload, GetConversation, MarkSession,
    PromptSession, QueueChange, QueueChanged, ReadAttachment, ResumeSession, Upload, WriteUpload,
    Written,
};
use vessel_proto::{
    Ack, AttachmentRef, BlobChunk, ContentBlock, Conversation, Delivered, ErrorCode, Mark,
    SessionInfo, WireError,
};

use super::{Connection, NodeService, wire};
use crate::error::ServiceError;
use crate::profiles::launch::creator;

/// What a call the wire has and this node does not do yet answers.
fn not_built(what: &str) -> WireError {
    WireError::new(
        ErrorCode::Unsupported,
        format!("{what} is not built on this node yet"),
    )
}

/// Attachments come with their slice of the track; until then a prompt
/// holding one is refused before anything is sent.
fn no_attachments(content: &[ContentBlock]) -> Result<(), WireError> {
    let attached = content
        .iter()
        .any(|block| matches!(block, ContentBlock::Attachment { .. }));
    if attached {
        return Err(not_built("attachments"));
    }
    Ok(())
}

#[expect(
    clippy::unused_async,
    reason = "the guard awaits every handler; these answer at once until \
              their slices of the track fill them"
)]
impl NodeService {
    /// `session.prompt`: deliver a prompt as asked; who sent it is kept
    /// with it if it waits in the queue.
    pub(crate) async fn prompt_session(
        &self,
        connection: &Connection,
        request: PromptSession,
    ) -> Result<Delivered, WireError> {
        no_attachments(&request.content)?;
        self.sessions
            .prompt(
                request.session,
                request.content,
                request.delivery,
                creator(&connection.principal),
            )
            .await
            .map_err(|error| wire(&error))
    }

    /// `session.queue`: change a session's queue.
    pub(crate) async fn change_queue(
        &self,
        _: &Connection,
        request: ChangeQueue,
    ) -> Result<QueueChanged, WireError> {
        if let QueueChange::Edit { content, .. } = &request.change {
            no_attachments(content)?;
        }
        self.sessions
            .change_queue(request.session, request.change)
            .await
            .map_err(|error| wire(&error))
    }

    /// `session.conversation`: what a structured session's log says now,
    /// without reading it.
    pub(crate) async fn get_conversation(
        &self,
        _: &Connection,
        request: GetConversation,
    ) -> Result<Conversation, WireError> {
        self.sessions
            .conversation(request.session)
            .await
            .map_err(|error| wire(&error))
    }

    pub(crate) async fn resume_session(
        &self,
        _: &Connection,
        _: ResumeSession,
    ) -> Result<SessionInfo, WireError> {
        Err(not_built("session.resume"))
    }

    /// `session.mark`: pin, archive (stopping a live session first), mark
    /// read or unread; the session's info after. Agents may not mark read
    /// or unread: an agent reading a session is not its owner seeing it.
    pub(crate) async fn mark_session(
        &self,
        connection: &Connection,
        request: MarkSession,
    ) -> Result<SessionInfo, WireError> {
        let MarkSession { session, mark } = request;
        let reading = match mark {
            Mark::Read { .. } | Mark::Unread => true,
            Mark::Pin | Mark::Unpin | Mark::Archive | Mark::Unarchive => false,
        };
        if reading && connection.principal().agent().is_some() {
            return Err(wire(&ServiceError::AgentMarksRead));
        }
        self.sessions
            .mark(session, mark)
            .await
            .map_err(|error| wire(&error))
    }

    /// `session.delete`: remove an ended session whole.
    pub(crate) async fn delete_session(
        &self,
        _: &Connection,
        request: DeleteSession,
    ) -> Result<Ack, WireError> {
        self.sessions
            .delete(request.session)
            .await
            .map(|()| Ack)
            .map_err(|error| wire(&error))
    }

    pub(crate) async fn begin_upload(
        &self,
        _: &Connection,
        _: BeginUpload,
    ) -> Result<Upload, WireError> {
        Err(not_built("attachment.begin"))
    }

    pub(crate) async fn write_upload(
        &self,
        _: &Connection,
        _: WriteUpload,
    ) -> Result<Written, WireError> {
        Err(not_built("attachment.write"))
    }

    pub(crate) async fn finish_upload(
        &self,
        _: &Connection,
        _: FinishUpload,
    ) -> Result<AttachmentRef, WireError> {
        Err(not_built("attachment.finish"))
    }

    pub(crate) async fn read_attachment(
        &self,
        _: &Connection,
        _: ReadAttachment,
    ) -> Result<BlobChunk, WireError> {
        Err(not_built("attachment.read"))
    }
}
