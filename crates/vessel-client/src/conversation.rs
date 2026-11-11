//! Protocol 10's calls on a session's conversation (milestone 11, track
//! A): what it holds now, and the calls that change it. Each is
//! [`NodeClient::call`] with its request; they live here rather than in
//! `client.rs`, which holds the calls of earlier protocols.

use vessel_proto::conversation::{
    ChangeQueue, DeleteSession, GetConversation, MarkSession, PromptSession, QueueChanged,
    ResumeSession,
};
use vessel_proto::{
    ContentBlock, Conversation, Delivered, Delivery, Mark, QueueChange, SessionId, SessionInfo,
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
    /// while its log is still being written (a retry works);
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
    /// marks, queue and attachments.
    ///
    /// # Errors
    /// `not_found` for an unknown session; `conflict` for a live one, or
    /// one whose log is still being written (a retry works); `unsupported`
    /// from a node before protocol 10; otherwise see [`CallError`].
    pub async fn delete_session(&self, session: SessionId) -> Result<(), CallError> {
        self.call(DeleteSession { session }).await.map(drop)
    }
}
