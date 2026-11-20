//! Capability lifetime follows the existing terminal entry and end watch.
//! Only hashes are retained in the registry; injected values are never logged.
use super::Terminals;
use tokio_util::sync::CancellationToken;
use vessel_proto::TerminalId;
use vessel_proto::notifications::{
    ATTENTION_NOTIFICATIONS_PROTOCOL, TerminalActivityAuth, TerminalActivitySecret,
};
use vessel_session::terminals::TerminalHandle;
use vessel_store::CredentialHash;

#[derive(Debug, Clone)]
pub(super) struct Producer {
    hash: CredentialHash,
    closing: CancellationToken,
}
impl Producer {
    fn accepts(&self, id: TerminalId, auth: &TerminalActivityAuth) -> bool {
        id == auth.terminal
            && !self.closing.is_cancelled()
            && self.hash.matches_terminal(&auth.secret)
    }
    pub(super) fn close(self) {
        self.closing.cancel();
    }
}
impl Terminals {
    /// The activity capability of a terminal about to be registered: part of
    /// its entry from the start, so registration cannot leave a live terminal
    /// without the capability its shell was given (owed item 4 of the node
    /// review of TRACKER 815). It closes when the terminal ends, the node
    /// stops, or the entry is forgotten.
    pub(super) fn producer_for(
        &self,
        handle: &TerminalHandle,
        secret: &TerminalActivitySecret,
    ) -> Producer {
        let closing = self.runner.shutdown.child_token();
        let ended = handle.clone();
        let close = closing.clone();
        drop(self.runner.tasks.spawn(async move {
            ended.ended().await;
            close.cancel();
        }));
        Producer {
            hash: CredentialHash::of_terminal(secret),
            closing,
        }
    }
    pub(crate) fn admit_producer(
        &self,
        auth: &TerminalActivityAuth,
    ) -> Option<(TerminalHandle, CancellationToken)> {
        let _assigned = ATTENTION_NOTIFICATIONS_PROTOCOL?;
        self.admit_assigned(auth)
    }
    /// [`admit_producer`](Self::admit_producer)'s checks behind the
    /// feature's gate: the exact terminal, still running, its own secret,
    /// and a capability not yet closed. Separate so that tests check them
    /// while the feature is unassigned (owed item 5 of the node review of
    /// TRACKER 815); the IPC route's own test needs the assigned protocol.
    pub(crate) fn admit_assigned(
        &self,
        auth: &TerminalActivityAuth,
    ) -> Option<(TerminalHandle, CancellationToken)> {
        let state = self.state();
        let entry = state.entries.get(&auth.terminal)?;
        let producer = entry.producer.as_ref()?;
        if entry.handle.is_ended() || !producer.accepts(entry.handle.id(), auth) {
            return None;
        }
        Some((entry.handle.clone(), producer.closing.child_token()))
    }
}
impl Terminals {
    /// Give a registered terminal an activity capability, as `register`
    /// does once the feature is assigned: the node gives none while it is
    /// unassigned, and tests check the capability's lifetime anyway.
    #[cfg(test)]
    pub(crate) fn give_producer(&self, id: TerminalId, secret: &TerminalActivitySecret) -> bool {
        let Ok(entry) = self.get(id) else {
            return false;
        };
        let producer = self.producer_for(&entry.handle, secret);
        let mut state = self.state();
        let Some(kept) = state.entries.get_mut(&id) else {
            return false;
        };
        kept.producer = Some(producer);
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credential_is_exact_terminal_constant_time_verified_redacted_and_closed_with_its_lifetime() {
        let secret = TerminalActivitySecret::from_bytes([7; 32]);
        let terminal = TerminalId::random();
        let auth = TerminalActivityAuth {
            terminal,
            secret: secret.clone(),
        };
        let producer = Producer {
            hash: CredentialHash::of_terminal(&secret),
            closing: CancellationToken::new(),
        };
        assert!(producer.accepts(terminal, &auth));
        assert!(!producer.accepts(TerminalId::random(), &auth));
        assert!(!producer.accepts(
            terminal,
            &TerminalActivityAuth {
                terminal,
                secret: TerminalActivitySecret::from_bytes([8; 32])
            }
        ));
        assert!(!format!("{producer:?} {auth:?}").contains(&secret.to_string()));
        producer.closing.cancel();
        assert!(!producer.accepts(terminal, &auth));
    }
}
