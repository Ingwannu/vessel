//! Protocol 10's calls on a session's conversation (milestone 11, track
//! A): what it holds now, and the calls that change it. Each is
//! [`NodeClient::call`] with its request; they live here rather than in
//! `client.rs`, which holds the calls of earlier protocols.

use vessel_proto::conversation::GetConversation;
use vessel_proto::{Conversation, SessionId};
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
}
