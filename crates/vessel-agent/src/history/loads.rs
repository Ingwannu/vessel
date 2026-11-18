//! A ranked listing's reads over ACP (ADR 0109): its pages as the cursor
//! listing reads them, and each conversation's `session/load` on the same
//! connection, its replay gathered by the connection's notification
//! handler (`super::over_acp`) into [`ranked::Gathered`].
//!
//! Kept apart from `history` only for size: the connection, its
//! `initialize` and its `session/list` are the parent's.

use std::path::PathBuf;
use std::sync::Arc;

use agent_client_protocol::v1::schema as acp;

use super::{
    Connection, HistoryEntry, Prompts, Ranked, RankedPage, Unlisted, ask_page, entry, initialize,
    ranked,
};

/// `initialize`, then the ranked listing over this connection.
pub(super) async fn ranked_over(
    cx: &Connection,
    cwd: Option<PathBuf>,
    ask: &Ranked,
    gathered: Arc<std::sync::Mutex<ranked::Gathered>>,
) -> Result<RankedPage, Unlisted> {
    let capabilities = initialize(cx).await?;
    let mut source = AcpSource {
        cx,
        cwd,
        gathered,
        closes: capabilities.session_capabilities.close.is_some(),
    };
    ranked::rank(&mut source, ask).await
}

/// A ranked listing's reads over ACP.
struct AcpSource<'a> {
    cx: &'a Connection,
    cwd: Option<PathBuf>,
    gathered: Arc<std::sync::Mutex<ranked::Gathered>>,
    /// Whether the agent takes `session/close`.
    closes: bool,
}

impl AcpSource<'_> {
    fn gathered(&self) -> std::sync::MutexGuard<'_, ranked::Gathered> {
        self.gathered
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl ranked::Source for AcpSource<'_> {
    async fn page(
        &mut self,
        cursor: Option<String>,
    ) -> Result<(Vec<HistoryEntry>, Option<String>), Unlisted> {
        let listed = ask_page(self.cx, self.cwd.clone(), cursor).await?;
        let entries = listed
            .sessions
            .into_iter()
            .filter(|session| session.cwd.is_absolute())
            .map(entry)
            .collect();
        Ok((entries, listed.next_cursor))
    }

    async fn load(&mut self, entry: &HistoryEntry) -> Option<Prompts> {
        let id = entry.session.as_str();
        self.gathered().begin(id);
        // No MCP servers and no prompt: the agent replays the conversation
        // and does nothing else, and this client answers no request of its.
        let loaded = self
            .cx
            .send_request(acp::LoadSessionRequest::new(
                acp::SessionId::new(id),
                &entry.cwd,
            ))
            .block_task()
            .await;
        match loaded {
            Ok(_loaded) => Some(self.gathered().finish(id)),
            Err(error) => {
                tracing::debug!(session = id, %error, "a listed conversation did not load; listed without its prompts");
                None
            }
        }
    }

    fn forget(&mut self, entry: &HistoryEntry) {
        self.gathered().forget(entry.session.as_str());
    }

    async fn close(&mut self, entry: &HistoryEntry) {
        if !self.closes {
            return;
        }
        let closed = self
            .cx
            .send_request(acp::CloseSessionRequest::new(acp::SessionId::new(
                entry.session.as_str(),
            )))
            .block_task()
            .await;
        if let Err(error) = closed {
            tracing::debug!(session = entry.session.as_str(), %error, "a loaded conversation did not close");
        }
    }
}
