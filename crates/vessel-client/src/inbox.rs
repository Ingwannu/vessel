//! Node-owned inbox calls (protocol 12, ADR 0040).
use crate::NodeClient;
use vessel_proto::inbox::{InboxKey, InboxPage, InboxRead, ListInbox, MarkInbox};
use vessel_rpc::CallError;
impl NodeClient {
    /// Read a bounded inbox page, newest first.
    ///
    /// # Errors
    /// `NotInProtocol` on an older node, or a transport/store failure.
    pub async fn inbox(&self, page: ListInbox) -> Result<InboxPage, CallError> {
        self.call(page).await
    }
    /// Mark exactly this current entry read or unread; never an answer.
    ///
    /// # Errors
    /// `forbidden` for agents, `not_found` once the entry leaves, or
    /// `NotInProtocol` on an older node; otherwise see [`CallError`].
    pub async fn mark_inbox(&self, key: InboxKey, read: InboxRead) -> Result<(), CallError> {
        self.call(MarkInbox { key, read }).await.map(|_ack| ())
    }
}

impl NodeClient {
    /// Recheck an entry after a delayed answer, independently of pagination.
    ///
    /// # Errors
    /// A protocol, transport or store failure; absence itself is `Ok(None)`.
    pub async fn inbox_entry(
        &self,
        key: InboxKey,
    ) -> Result<Option<vessel_proto::inbox::InboxEntry>, CallError> {
        self.call(vessel_proto::inbox::GetInboxEntry {
            key,
            fetch: vessel_proto::inbox::InboxFetch::Summary,
        })
        .await
    }
}

impl NodeClient {
    /// Recover just this live form after log pruning, without fetching every
    /// pending request. An over-limit form remains an explicit compact entry.
    ///
    /// # Errors
    /// Protocol/transport/store failure; absence means answered or withdrawn.
    pub async fn inbox_request(
        &self,
        key: InboxKey,
    ) -> Result<Option<vessel_proto::inbox::InboxEntry>, CallError> {
        self.call(vessel_proto::inbox::GetInboxEntry {
            key,
            fetch: vessel_proto::inbox::InboxFetch::LiveRequest,
        })
        .await
    }
}
