//! Protocol 10's calls (milestone 11's Track A, the conversation):
//! delivering a prompt, the queue, the conversation as it stands, starting
//! an ended session again, marks, deleting, and attachments. Routing only,
//! as the rest of the service.

use vessel_proto::conversation::{
    BeginUpload, ChangeQueue, DeleteSession, FinishUpload, GetConversation, MarkSession,
    PromptSession, QueueChanged, ReadAttachment, ResumeSession, SessionDeleted, Upload,
    WriteUpload, Written,
};
use vessel_proto::{
    AttachmentRef, BlobChunk, Conversation, Delivered, Mark, SessionId, SessionInfo, WireError,
};

use super::{Connection, NodeService, wire};
use crate::error::ServiceError;
use crate::profiles::launch::creator;
use crate::transfers::Holder;

impl NodeService {
    /// `session.prompt`: deliver a prompt as asked; who sent it is kept
    /// with it if it waits in the queue.
    pub(crate) async fn prompt_session(
        &self,
        connection: &Connection,
        request: PromptSession,
    ) -> Result<Delivered, WireError> {
        if request.notify {
            return self.prompt_told(connection, request).await;
        }
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

    /// `session.resume`: start an ended structured session again, its log
    /// continued; answers once its agent is ready (`again`).
    pub(crate) async fn resume_session(
        &self,
        connection: &Connection,
        request: ResumeSession,
    ) -> Result<SessionInfo, WireError> {
        self.start_again(connection, request.session, None)
            .await
            .map(|info| self.with_workspace(info))
            .map_err(|error| wire(&error))
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
        let marked = match mark {
            Mark::Archive => self.put_away(session).await,
            Mark::Read { .. } | Mark::Unread | Mark::Pin | Mark::Unpin | Mark::Unarchive => {
                self.sessions.mark(session, mark).await
            }
        };
        marked
            .map(|info| self.with_workspace(info))
            .map_err(|error| wire(&error))
    }

    /// Archive `session` (stopping it first when live) and end what runs
    /// beside it: the one way a session is put away, a person's
    /// `session.mark` and a schedule's run after its first turn (protocol
    /// 11) alike.
    ///
    /// An archived session is not watched, and its terminals and services
    /// end (kept, with their output): a service beside a session put away
    /// would hold its port and its worktree unseen ("Integration 2
    /// (node)", meeting point 3). A schedule's session must not outlive
    /// what a person's own would ("Integration 3 (node)").
    pub(super) async fn put_away(&self, session: SessionId) -> Result<SessionInfo, ServiceError> {
        let archived = self.sessions.mark(session, Mark::Archive).await?;
        self.unwatch_session(session);
        self.terminals.end_beside(session).await;
        Ok(archived)
    }

    /// `session.delete`: remove an ended session whole, and the worktree
    /// made for it by the rule that keeps work (`tidying`).
    pub(crate) async fn delete_session(
        &self,
        _: &Connection,
        request: DeleteSession,
    ) -> Result<SessionDeleted, WireError> {
        let deleted = self
            .tidying
            .delete(request.session)
            .await
            .map_err(|error| wire(&error))?;
        self.unwatch_session(request.session);
        Ok(deleted)
    }

    /// `attachment.begin`: declare a file for a live structured session;
    /// the upload is the caller's.
    pub(crate) async fn begin_upload(
        &self,
        connection: &Connection,
        request: BeginUpload,
    ) -> Result<Upload, WireError> {
        self.attachments
            .begin(Holder::of(connection.principal()), request)
            .await
            .map_err(|error| wire(&error))
    }

    /// `attachment.write`: the next piece of the caller's upload.
    pub(crate) async fn write_upload(
        &self,
        connection: &Connection,
        request: WriteUpload,
    ) -> Result<Written, WireError> {
        self.attachments
            .write(Holder::of(connection.principal()), request)
            .await
            .map_err(|error| wire(&error))
    }

    /// `attachment.finish`: keep the caller's upload with its session.
    pub(crate) async fn finish_upload(
        &self,
        connection: &Connection,
        request: FinishUpload,
    ) -> Result<AttachmentRef, WireError> {
        self.attachments
            .finish(Holder::of(connection.principal()), request.upload)
            .await
            .map_err(|error| wire(&error))
    }

    /// `attachment.read`: a piece of a session's attachment.
    pub(crate) async fn read_attachment(
        &self,
        _: &Connection,
        request: ReadAttachment,
    ) -> Result<BlobChunk, WireError> {
        self.attachments
            .read(request)
            .await
            .map_err(|error| wire(&error))
    }
}
