//! Protocol 10's calls on a session's conversation (milestone 11, track
//! A): what it holds now, and the calls that change it. Each is
//! [`NodeClient::call`] with its request; they live here rather than in
//! `client.rs`, which holds the calls of earlier protocols.

use vessel_proto::conversation::{ChangeQueue, GetConversation, PromptSession, QueueChanged};
use vessel_proto::{ContentBlock, Conversation, Delivered, Delivery, QueueChange, SessionId};
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
}
