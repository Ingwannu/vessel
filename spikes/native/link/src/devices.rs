//! Typed device RPCs. Failures cross into the view by code only: even a
//! faulty peer echoing an invitation in its error cannot disclose it.
//!
//! [Decision Log]
//! - Purpose: list, invite and revoke through the existing connection.
//! - Constraints: a pending request must not hold the fleet task; a
//!   credential must never enter logs, files or a raw error string.
//! - Alternatives: calls in the screen; forwarding full RPC error text.
//! - Choice: one typed ask/answer seam, with request identity, and safe
//!   failure projection. Raw errors are deliberately not retained across
//!   this trust boundary; only their structured code is meaningful here.
//! - Cost: unusual peer failures use a general localized explanation.

use vessel_app_view::devices::{MachineKey, Pairing, Problem, Ticket};
use vessel_proto::{DeviceId, DeviceInfo};

/// What the Devices page asks a node.
#[derive(Debug)]
pub enum Ask {
    /// List all devices, including revoked ones.
    List,
    /// Make a single-use invitation using the page's ten-minute lifetime.
    Invite {
        /// Optional, already trimmed device label.
        label: Option<String>,
    },
    /// Revoke after the person confirmed the effect.
    Revoke {
        /// The device to revoke.
        device: DeviceId,
    },
}

/// A response tagged with the outstanding page request's identity.
#[derive(Debug)]
pub enum Answer {
    /// The current devices.
    Listed(Ticket, Result<Vec<DeviceInfo>, Problem>),
    /// A credential, initially masked by its view model.
    Invited(Ticket, Result<Pairing, Problem>),
    /// The confirmed revocation's result.
    Revoked(Ticket, Result<(), Problem>),
}

impl Ask {
    /// Refuse without a connection (or on the browser's demo surface).
    #[must_use]
    pub fn refused(self, ticket: Ticket, why: Problem) -> Answer {
        match self {
            Self::List => Answer::Listed(ticket, Err(why)),
            Self::Invite { .. } => Answer::Invited(ticket, Err(why)),
            Self::Revoke { .. } => Answer::Revoked(ticket, Err(why)),
        }
    }
}

impl crate::Link {
    /// Queue a Devices action, or return a safe failure immediately.
    /// Unlike the general fire-and-forget path, a rejected request must
    /// settle the page's pending state rather than leave it spinning.
    ///
    /// # Errors
    /// The queue is full, closed, or this is the browser demo surface.
    pub fn request_devices(
        &self,
        machine: MachineKey,
        ticket: Ticket,
        ask: Ask,
    ) -> Result<(), Problem> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            enqueue(&self.commands, machine, ticket, ask)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _request = (machine, ticket, ask);
            Err(Problem::Refused(vessel_proto::ErrorCode::Unsupported))
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn enqueue(
    commands: &tokio::sync::mpsc::Sender<crate::Command>,
    machine: MachineKey,
    ticket: Ticket,
    ask: Ask,
) -> Result<(), Problem> {
    crate::offer(
        commands,
        crate::Command::Devices {
            machine,
            ticket,
            ask,
        },
    )
    .map_err(|unsent| rejected(unsent.why))
}

/// What a device page says of a request the link did not take: a full
/// queue as the node's `unavailable` (try again), a stopped link as
/// offline.
#[must_use]
pub const fn rejected(why: crate::Rejected) -> Problem {
    match why {
        crate::Rejected::Full => Problem::Refused(vessel_proto::ErrorCode::Unavailable),
        crate::Rejected::Closed => Problem::Offline,
    }
}

/// Calls are bounded even while a person switches machines repeatedly.
/// A stalled node cannot accumulate one background task per navigation.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub(crate) struct Hub(std::sync::Arc<tokio::sync::Semaphore>);

#[cfg(not(target_arch = "wasm32"))]
impl Default for Hub {
    fn default() -> Self {
        Self(std::sync::Arc::new(tokio::sync::Semaphore::new(8)))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    pub(crate) async fn devices(&self, machine: &MachineKey, ticket: Ticket, ask: Ask) {
        let client = match self.client(machine.as_str()) {
            Ok(client) => client.clone(),
            Err(_not_connected) => {
                self.emit(crate::Event::Devices(ask.refused(ticket, Problem::Offline)))
                    .await;
                return;
            }
        };
        let Ok(permit) = std::sync::Arc::clone(&self.device_calls.0).try_acquire_owned() else {
            self.emit(crate::Event::Devices(ask.refused(
                ticket,
                Problem::Refused(vessel_proto::ErrorCode::Unavailable),
            )))
            .await;
            return;
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let answer = bounded_call(&client, ticket, ask).await;
            let _closed = events.send(crate::Event::Devices(answer)).await;
            // The permit covers the event send too: a stopped screen's
            // full queue cannot accumulate tasks holding completed replies.
            drop(permit);
        }));
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn bounded_call(client: &vessel_client::NodeClient, ticket: Ticket, ask: Ask) -> Answer {
    // Keep the operation kind to answer a timeout without retaining a
    // credential or cloning a request. A timed-out write may have taken
    // effect; the page refreshes instead of applying an optimistic edit.
    let timed_out = match &ask {
        Ask::List => Answer::Listed(
            ticket,
            Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable)),
        ),
        Ask::Invite { .. } => Answer::Invited(
            ticket,
            Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable)),
        ),
        Ask::Revoke { .. } => Answer::Revoked(
            ticket,
            Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable)),
        ),
    };
    tokio::time::timeout(
        std::time::Duration::from_secs(15),
        call(client, ticket, ask),
    )
    .await
    .unwrap_or(timed_out)
}

#[cfg(not(target_arch = "wasm32"))]
async fn call(client: &vessel_client::NodeClient, ticket: Ticket, ask: Ask) -> Answer {
    match ask {
        Ask::List => Answer::Listed(ticket, client.devices().await.map_err(problem)),
        Ask::Revoke { device } => {
            Answer::Revoked(ticket, client.revoke_device(device).await.map_err(problem))
        }
        Ask::Invite { label } => {
            let result = match client
                .invite_device(label, vessel_app_view::devices::LINK_TTL_SECS)
                .await
            {
                Ok(invitation) => match invitation.link.decode() {
                    Ok(offer) => vessel_app_view::devices::invitation_route(&offer)
                        .map(|()| Pairing::new(invitation, vessel_client::fingerprint(&offer.key))),
                    Err(_invalid) => Err(Problem::InvalidReply),
                },
                Err(error) => Err(problem(error)),
            };
            Answer::Invited(ticket, result)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn problem(error: vessel_client::CallError) -> Problem {
    match error {
        vessel_client::CallError::Disconnected => Problem::Offline,
        vessel_client::CallError::Remote(error) => Problem::Refused(error.code),
        vessel_client::CallError::NotInProtocol { .. }
        | vessel_client::CallError::UnassignedProtocol { .. } => {
            Problem::Refused(vessel_proto::ErrorCode::Unsupported)
        }
        vessel_client::CallError::UnexpectedReply(_) => Problem::InvalidReply,
        vessel_client::CallError::RequestIdsExhausted => Problem::Exhausted,
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    #[test]
    fn an_unassigned_call_is_unsupported() {
        assert_eq!(
            problem(vessel_client::CallError::UnassignedProtocol {
                method: vessel_proto::Method::NodeInfo,
            }),
            Problem::Refused(vessel_proto::ErrorCode::Unsupported)
        );
    }
    #[test]
    fn a_peer_cannot_echo_a_credential_through_an_error_or_debug() {
        let raw = "a-credential-that-a-faulty-peer-echoed";
        let error = vessel_client::CallError::Remote(vessel_proto::WireError::new(
            vessel_proto::ErrorCode::Forbidden,
            raw,
        ));
        let safe = problem(error);
        assert!(!format!("{safe:?}").contains(raw));
        assert!(!format!("{safe}").contains(raw));
        assert_eq!(safe, Problem::Refused(vessel_proto::ErrorCode::Forbidden));
        assert_eq!(
            problem(vessel_client::CallError::Disconnected),
            Problem::Offline
        );
        assert_eq!(
            problem(vessel_client::CallError::RequestIdsExhausted),
            Problem::Exhausted
        );
    }
    #[test]
    fn a_full_or_closed_command_queue_is_an_answerable_failure() {
        let (commands, inbox) = tokio::sync::mpsc::channel(1);
        let mut tickets = vessel_app_view::devices::Tickets::default();
        let machine = MachineKey::from_book("laptop");
        enqueue(
            &commands,
            machine.clone(),
            tickets.next_ticket().unwrap(),
            Ask::List,
        )
        .unwrap();
        assert_eq!(
            enqueue(
                &commands,
                machine.clone(),
                tickets.next_ticket().unwrap(),
                Ask::List
            ),
            Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable))
        );
        drop(inbox);
        assert_eq!(
            enqueue(
                &commands,
                machine,
                tickets.next_ticket().unwrap(),
                Ask::List
            ),
            Err(Problem::Offline)
        );
    }

    #[test]
    fn device_calls_have_a_hard_concurrency_bound() {
        let hub = Hub::default();
        let permits: Vec<_> = (0..8)
            .map(|_| std::sync::Arc::clone(&hub.0).try_acquire_owned().unwrap())
            .collect();
        assert!(std::sync::Arc::clone(&hub.0).try_acquire_owned().is_err());
        drop(permits);
        assert!(std::sync::Arc::clone(&hub.0).try_acquire_owned().is_ok());
    }
}
