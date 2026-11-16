//! Identity-scoped reads and stream opens cannot follow a replaced saved alias.
use crate::{Act, ActAnswer, ActFailed, Event};
use vessel_app_view::{machine_book::Name, sessions::SessionAddress};

/// An authenticated destination retained after deep-link lookup.
#[derive(Debug, Clone)]
pub struct Binding {
    /// Current validated node-book name, never enough on its own for authority.
    pub host: Name,
    /// The node identity and exact session expected on that connection.
    pub address: SessionAddress,
}
/// Only passive session reads are accepted on the navigation hydration path.
#[derive(Debug, Clone, Copy)]
pub enum Read {
    /// The session's current metadata.
    Info,
    /// The session's current conversation projection.
    Conversation,
}
impl Read {
    fn act(self) -> Act {
        match self {
            Self::Info => Act::Info,
            Self::Conversation => Act::Conversation,
        }
    }
}
/// A passive operation bound to the node authenticated during navigation.
#[derive(Debug)]
pub enum Access {
    /// Hydrate metadata or pending conversation state.
    Read {
        /// Authenticated target retained by the opened session.
        binding: Binding,
        /// Which passive projection is needed.
        read: Read,
    },
    /// Follow the existing log without starting a provider.
    Follow(Binding),
}
impl Access {
    #[cfg(not(target_arch = "wasm32"))]
    fn binding(&self) -> &Binding {
        match self {
            Self::Read { binding, .. } | Self::Follow(binding) => binding,
        }
    }
    /// Queue/transport refusal still settles the exact screen read or stream.
    #[must_use]
    pub fn refused(self, failure: ActFailed) -> Event {
        match self {
            Self::Read { binding, read } => Event::Acted(ActAnswer {
                terminal: None,
                machine: binding.host.as_str().to_owned(),
                session: binding.address.session,
                kind: read.act().kind(),
                result: Err(failure),
            }),
            Self::Follow(binding) => Event::SessionLost {
                session: binding.address.session,
                why: failure.message,
            },
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    pub(crate) fn session_access(&mut self, access: Access) {
        let binding = access.binding();
        let failure = match self.clients.get(binding.host.as_str()) {
            Some(connected) if connected.client.node().id == binding.address.node => None,
            Some(_) => Some(ActFailed {
                code: Some(vessel_proto::ErrorCode::Conflict),
                message: "The authenticated session destination changed".to_owned(),
            }),
            None => Some(ActFailed {
                code: None,
                message: crate::LinkError::NotConnected(binding.host.as_str().to_owned())
                    .to_string(),
            }),
        };
        if let Some(failure) = failure {
            let events = self.events.clone();
            drop(tokio::spawn(async move {
                let _closed = events.send(access.refused(failure)).await;
            }));
            return;
        }
        // Both lookup and capture happen synchronously in the link actor. No
        // refresh can replace the validated client between these two operations.
        match access {
            Access::Read { binding, read } => {
                self.act(binding.host.as_str(), binding.address.session, read.act());
            }
            Access::Follow(binding) => self.open(binding.host.as_str(), binding.address.session),
        }
    }
}
