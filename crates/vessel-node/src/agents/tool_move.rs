//! A replacement launch can gain or lose built-in tools. Its credential
//! is staged beside the old one until the operation owns the live run.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use vessel_proto::{AgentSecret, SessionId};
use vessel_store::CredentialHash;

use super::{Agents, ServiceError, mint};

#[derive(Debug)]
pub(super) struct PendingTools {
    ticket: u64,
    credential: Option<CredentialHash>,
}

impl PendingTools {
    pub(super) fn credential(&self) -> Option<&CredentialHash> {
        self.credential.as_ref()
    }
}

pub(crate) struct ToolMove {
    agents: Arc<Agents>,
    session: SessionId,
    ticket: u64,
}

impl Agents {
    /// Prepare optional credentials for a replacement. Until settlement
    /// either running process can connect with only its existing grant.
    pub(crate) fn begin_move(
        self: &Arc<Self>,
        session: SessionId,
        inject: bool,
    ) -> Result<(Option<ToolMove>, Option<AgentSecret>), ServiceError> {
        let mut sessions = self.lock();
        let Some(entry) = sessions.get_mut(&session) else {
            return Ok((None, None));
        };
        if entry.pending_tools.is_some() {
            return Err(ServiceError::MoveRunning(session));
        }
        let secret = inject.then(mint).transpose()?;
        let ticket = self.next.fetch_add(1, Ordering::Relaxed);
        entry.pending_tools = Some(PendingTools {
            ticket,
            credential: secret.as_ref().map(CredentialHash::of_agent),
        });
        Ok((
            Some(ToolMove {
                agents: Arc::clone(self),
                session,
                ticket,
            }),
            secret,
        ))
    }
}

impl ToolMove {
    pub(crate) fn finish(self, moved: bool) {
        self.settle(moved);
    }

    fn settle(&self, moved: bool) {
        let mut sessions = self.agents.lock();
        let Some(entry) = sessions.get_mut(&self.session) else {
            return;
        };
        let Some(pending) = entry
            .pending_tools
            .take_if(|pending| pending.ticket == self.ticket)
        else {
            return;
        };
        if moved {
            entry.credential = pending.credential;
        }
        if entry.credential.is_none() {
            // A process admitted during the transition must not retain a
            // connection after the session has no credential.
            for (_, closing) in entry.connections.drain() {
                closing.cancel();
            }
        }
    }
}

impl Drop for ToolMove {
    fn drop(&mut self) {
        // An operation that returns early can only undo its own ticket.
        self.settle(false);
    }
}
