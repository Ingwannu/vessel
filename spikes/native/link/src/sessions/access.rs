//! Identity-scoped reads and stream opens cannot follow a replaced saved alias.
use crate::{Act, ActAnswer, ActFailed, ActKind, Event, TerminalOpening};
use vessel_app_view::timeline::Until;
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
    /// Read terminal history while preserving its opaque opening receipt.
    ReadTerminal {
        /// Originally admitted node/session identity; required even when offline.
        binding: Binding,
        /// The exact retained frontend opening, echoed on success and refusal.
        opening: TerminalOpening,
        /// The existing bounded log cutoff.
        until: Until,
    },
}
impl Access {
    #[cfg(not(target_arch = "wasm32"))]
    fn binding(&self) -> &Binding {
        match self {
            Self::Read { binding, .. }
            | Self::ReadTerminal { binding, .. }
            | Self::Follow(binding) => binding,
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
            Self::ReadTerminal { opening, .. } => Event::Acted(ActAnswer {
                machine: opening.machine().as_str().to_owned(),
                session: opening.session(),
                terminal: Some(opening),
                kind: ActKind::Log,
                result: Err(failure),
            }),
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    pub(crate) fn session_access(&mut self, access: Access) {
        let failure = self.session_access_failure(&access);
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
            Access::ReadTerminal {
                binding,
                opening,
                until,
            } => self.act(
                binding.host.as_str(),
                binding.address.session,
                Act::TerminalLog { opening, until },
            ),
        }
    }
    fn session_access_failure(&self, access: &Access) -> Option<ActFailed> {
        let binding = access.binding();
        let receipt_matches = match access {
            Access::ReadTerminal {
                binding, opening, ..
            } => {
                binding.host.as_str() == opening.machine().as_str()
                    && binding.address.session == opening.session()
            }
            Access::Read { .. } | Access::Follow(_) => true,
        };
        if receipt_matches {
            match self.clients.get(binding.host.as_str()) {
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
            }
        } else {
            Some(ActFailed {
                code: Some(vessel_proto::ErrorCode::Conflict),
                message: "The terminal opening does not match its authenticated destination"
                    .to_owned(),
            })
        }
    }
}
