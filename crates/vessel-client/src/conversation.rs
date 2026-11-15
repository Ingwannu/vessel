//! Protocol 10's calls on a session's conversation (milestone 11, track
//! A): what it holds now, and the calls that change it. Each is
//! [`NodeClient::call`] with its request; they live here rather than in
//! `client.rs`, which holds the calls of earlier protocols.

use bytes::Bytes;
use vessel_proto::conversation::{
    BeginUpload, ChangeQueue, DeleteSession, FinishUpload, GetConversation, MarkSession, MimeType,
    PromptSession, QueueChanged, ReadAttachment, ResumeSession, SessionDeleted, Upload,
    WriteUpload, Written,
};
use vessel_proto::{
    AttachmentId, AttachmentName, AttachmentRef, BlobChunk, ContentBlock, Conversation, Delivered,
    Delivery, Mark, QuestionAnswer, QuestionId, QueueChange, SessionId, SessionInfo, SessionInput,
    UploadId,
};
use vessel_rpc::CallError;

use crate::NodeClient;

impl NodeClient {
    /// `session.conversation` (protocol 10): a structured session's queue,
    /// the agent's commands and context, its abilities, the permission
    /// requests and questions waiting, and the attachment limits.
    ///
    /// # Errors
    /// `NotFound` for an unknown session; `Unsupported` for a terminal
    /// session or a node before protocol 10; otherwise see [`CallError`].
    pub async fn conversation(&self, session: SessionId) -> Result<Conversation, CallError> {
        self.call(GetConversation { session }).await
    }

    /// `session.prompt` (protocol 10): send a prompt, delivered as asked.
    ///
    /// # Errors
    /// `invalid_request` for an empty prompt; `conflict` while a turn runs
    /// (`start`), for a full queue or an ended session; `unsupported` for
    /// a steer the agent cannot take, a terminal session or a node before
    /// protocol 10; `unavailable` when an interrupted turn did not end;
    /// otherwise see [`CallError`].
    pub async fn prompt(
        &self,
        session: SessionId,
        content: Vec<ContentBlock>,
        delivery: Delivery,
    ) -> Result<Delivered, CallError> {
        self.call(PromptSession {
            session,
            content,
            delivery,
            notify: false,
        })
        .await
    }

    /// `session.prompt { notify }` (protocol 11): as
    /// [`prompt`](Self::prompt), and the caller's own session is told,
    /// through its queue, when the turn this prompt starts waits for an
    /// answer and when it finishes, or the session ends. Only a session's
    /// agent asks, on its own node.
    ///
    /// # Errors
    /// As [`prompt`](Self::prompt); `invalid_request` for a caller that is
    /// not a session's agent, `forbidden` through another node,
    /// `unsupported` for a terminal caller, `conflict` past the bounds of
    /// turns followed.
    pub async fn prompt_told(
        &self,
        session: SessionId,
        content: Vec<ContentBlock>,
        delivery: Delivery,
    ) -> Result<Delivered, CallError> {
        self.call(PromptSession {
            session,
            content,
            delivery,
            notify: true,
        })
        .await
    }

    /// `session.queue` (protocol 10): change a session's queue; the queue
    /// after, and how an entry sent now went.
    ///
    /// # Errors
    /// `not_found` for an entry no longer queued; `conflict` for an edit
    /// from another revision or an ended session; a `send_now`'s delivery
    /// errors, as [`prompt`](Self::prompt); otherwise see [`CallError`].
    pub async fn change_queue(
        &self,
        session: SessionId,
        change: QueueChange,
    ) -> Result<QueueChanged, CallError> {
        self.call(ChangeQueue { session, change }).await
    }

    /// `session.send`'s `answer_question` (protocol 10): answer a question
    /// the session's agent asks; the first answer wins.
    ///
    /// # Errors
    /// `invalid_request` for an answer that does not fit the question's
    /// form; `not_found` for a question the session does not have waiting;
    /// `conflict` once it is settled; `unsupported` from a node before
    /// protocol 10; otherwise see [`CallError`].
    pub async fn answer_question(
        &self,
        session: SessionId,
        question: QuestionId,
        answer: QuestionAnswer,
    ) -> Result<(), CallError> {
        self.send(
            session,
            SessionInput::AnswerQuestion {
                id: question,
                answer,
            },
        )
        .await
    }

    /// `session.mark` (protocol 10): pin, archive (a live session is
    /// stopped first, and the call answers once it has ended), mark read
    /// or unread; the session's info after, with its marks.
    ///
    /// # Errors
    /// `not_found` for an unknown session; `invalid_request` for a `read`
    /// past the end of the log; `forbidden` for an agent marking read or
    /// unread; `unsupported` from a node before protocol 10; otherwise see
    /// [`CallError`].
    pub async fn mark(&self, session: SessionId, mark: Mark) -> Result<SessionInfo, CallError> {
        self.call(MarkSession { session, mark }).await
    }

    /// `session.resume` (protocol 10): start an ended structured session
    /// again, its log continued after its final status, its queue back
    /// held, out of the archive; its info once its agent is ready.
    ///
    /// # Errors
    /// `not_found` for an unknown session; `conflict` while it is live, or
    /// while its log is still being written after the node waited for it
    /// (a retry works);
    /// `unsupported` for a terminal session, an agent that cannot resume,
    /// a session that never had a conversation to resume, or a node
    /// before protocol 10; `invalid_request` when its profile is gone;
    /// `harness_not_installed`; `spawn_failed` when the agent did not
    /// start or could not load the conversation (its log says why);
    /// `unavailable` while the node drains; `forbidden` past an agent's
    /// limits; otherwise see [`CallError`].
    pub async fn resume_session(&self, session: SessionId) -> Result<SessionInfo, CallError> {
        self.call(ResumeSession { session }).await
    }

    /// `session.delete` (protocol 10): remove an ended session, its log,
    /// marks, queue and attachments; the answer says what became of the
    /// worktree made for it (removed, or kept and why).
    ///
    /// # Errors
    /// `not_found` for an unknown session; `conflict` for a live one, or
    /// one whose log is still being written after the node waited for it
    /// (a retry works); `unsupported`
    /// from a node before protocol 10; otherwise see [`CallError`].
    pub async fn delete_session(&self, session: SessionId) -> Result<SessionDeleted, CallError> {
        self.call(DeleteSession { session }).await
    }

    /// `attachment.begin` (protocol 10): declare a file of `size` bytes
    /// for a live structured session; the upload, and the largest piece
    /// [`write_upload`](Self::write_upload) takes.
    ///
    /// # Errors
    /// `not_found` for an unknown session; `unsupported` for a terminal
    /// session, a kind its agent does not take, or a node before protocol
    /// 10; `conflict` once it ended; `invalid_request` past a size limit;
    /// `unavailable` while too many uploads are open; otherwise see
    /// [`CallError`].
    pub async fn begin_upload(
        &self,
        session: SessionId,
        name: AttachmentName,
        mime: MimeType,
        size: u64,
    ) -> Result<Upload, CallError> {
        self.call(BeginUpload {
            session,
            name,
            mime,
            size,
        })
        .await
    }

    /// `attachment.write` (protocol 10): the piece of `upload` at `offset`
    /// (what was received so far); how much the node has after it. A
    /// piece sent again after a lost reply is answered without being
    /// written twice.
    ///
    /// # Errors
    /// `not_found` for an upload this caller did not begin (or one that
    /// was dropped); `invalid_request` for a piece out of order, too large
    /// or past the declared size; otherwise see [`CallError`].
    pub async fn write_upload(
        &self,
        upload: UploadId,
        offset: u64,
        data: Bytes,
    ) -> Result<Written, CallError> {
        self.call(WriteUpload {
            upload,
            offset,
            data,
        })
        .await
    }

    /// `attachment.finish` (protocol 10): keep `upload` with its session;
    /// the reference a prompt names it by.
    ///
    /// # Errors
    /// `not_found` as for [`write_upload`](Self::write_upload);
    /// `invalid_request` when bytes are missing or an image does not start
    /// as its type does; `conflict` once the session ended; otherwise see
    /// [`CallError`].
    pub async fn finish_upload(&self, upload: UploadId) -> Result<AttachmentRef, CallError> {
        self.call(FinishUpload { upload }).await
    }

    /// Upload `data` as an attachment of `session`: begin, every piece in
    /// order, finish.
    ///
    /// # Errors
    /// As [`begin_upload`](Self::begin_upload),
    /// [`write_upload`](Self::write_upload) and
    /// [`finish_upload`](Self::finish_upload).
    pub async fn attach(
        &self,
        session: SessionId,
        name: AttachmentName,
        mime: MimeType,
        data: Bytes,
    ) -> Result<AttachmentRef, CallError> {
        let size = u64::try_from(data.len()).unwrap_or(u64::MAX);
        let Upload { upload, chunk } = self.begin_upload(session, name, mime, size).await?;
        let chunk = usize::try_from(chunk).unwrap_or(usize::MAX).max(1);
        let mut offset = 0_usize;
        while offset < data.len() {
            let end = offset.saturating_add(chunk).min(data.len());
            let at = u64::try_from(offset).unwrap_or(u64::MAX);
            let _written = self
                .write_upload(upload, at, data.slice(offset..end))
                .await?;
            offset = end;
        }
        self.finish_upload(upload).await
    }

    /// `attachment.read` (protocol 10): a piece of one of `session`'s
    /// attachments from `offset`, with its whole size.
    ///
    /// # Errors
    /// `not_found` for an unknown session or attachment; `invalid_request`
    /// past its end; `conflict` when its file was changed on the node;
    /// `unsupported` for a terminal session or a node before protocol 10;
    /// otherwise see [`CallError`].
    pub async fn read_attachment(
        &self,
        session: SessionId,
        attachment: AttachmentId,
        offset: u64,
    ) -> Result<BlobChunk, CallError> {
        self.call(ReadAttachment {
            session,
            attachment,
            offset,
        })
        .await
    }
}
