//! Capability lifetime follows the existing terminal entry and end watch.
//! Only hashes are retained in the registry; injected values are never logged.
use super::{Terminals, TerminalsError};
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
    pub(crate) fn install_producer(
        &self,
        id: TerminalId,
        secret: &TerminalActivitySecret,
    ) -> Result<(), TerminalsError> {
        let (handle, closing) = {
            let mut state = self.state();
            let entry = state
                .entries
                .get_mut(&id)
                .ok_or(TerminalsError::NotFound(id))?;
            if entry.producer.is_some() {
                return Err(TerminalsError::Conflict(
                    "terminal activity credential is already installed".to_owned(),
                ));
            }
            let closing = self.runner.shutdown.child_token();
            entry.producer = Some(Producer {
                hash: CredentialHash::of_terminal(secret),
                closing: closing.clone(),
            });
            (entry.handle.clone(), closing)
        };
        drop(self.runner.tasks.spawn(async move {
            handle.ended().await;
            closing.cancel();
        }));
        Ok(())
    }
    pub(crate) fn admit_producer(
        &self,
        auth: &TerminalActivityAuth,
    ) -> Option<(TerminalHandle, CancellationToken)> {
        let _assigned = ATTENTION_NOTIFICATIONS_PROTOCOL?;
        let state = self.state();
        let entry = state.entries.get(&auth.terminal)?;
        let producer = entry.producer.as_ref()?;
        if entry.handle.is_ended() || !producer.accepts(entry.handle.id(), auth) {
            return None;
        }
        Some((entry.handle.clone(), producer.closing.child_token()))
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
