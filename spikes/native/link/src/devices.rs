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

use vessel_app_view::devices::{ConnectionContext, Pairing, Problem, Ticket};
use vessel_proto::{DeviceId, DeviceInfo};

/// What the Devices page asks a node.
#[derive(Debug)]
pub enum Ask {
    /// Inspect only this admitted caller’s effective grant and optional id.
    Authority,
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
pub enum Outcome {
    /// A sent write hit the local deadline; its remote outcome is unknown.
    Unconfirmed(Ticket),
    /// Facts of this exact authenticated caller, never a guessed default grant.
    Authority(Ticket, Result<vessel_proto::DeviceSelf, Problem>),
    /// The current devices.
    Listed(Ticket, Result<Vec<DeviceInfo>, Problem>),
    /// A credential, initially masked by its view model.
    Invited(Ticket, Result<Pairing, Problem>),
    /// The confirmed revocation's result.
    Revoked(Ticket, Result<(), Problem>),
}

mod receipt;
use receipt::Pending;
pub use receipt::{Answer, Failed, Received};

#[derive(Clone, Copy)]
enum Kind {
    Authority,
    List,
    Invite,
    Revoke,
}
impl Kind {
    #[cfg(not(target_arch = "wasm32"))]
    fn timed_out(self, ticket: Ticket) -> Outcome {
        match self {
            Self::Invite | Self::Revoke => Outcome::Unconfirmed(ticket),
            Self::Authority | Self::List => self.refused(
                ticket,
                Problem::Refused(vessel_proto::ErrorCode::Unavailable),
            ),
        }
    }
    fn refused(self, ticket: Ticket, why: Problem) -> Outcome {
        match self {
            Self::Authority => Outcome::Authority(ticket, Err(why)),
            Self::List => Outcome::Listed(ticket, Err(why)),
            Self::Invite => Outcome::Invited(ticket, Err(why)),
            Self::Revoke => Outcome::Revoked(ticket, Err(why)),
        }
    }
}

impl Ask {
    fn kind(&self) -> Kind {
        match self {
            Self::Authority => Kind::Authority,
            Self::List => Kind::List,
            Self::Invite { .. } => Kind::Invite,
            Self::Revoke { .. } => Kind::Revoke,
        }
    }
    /// Correlate a local refusal without any successful data or guard.
    /// This is used only when a command was never dispatched.
    #[must_use]
    pub fn rejected(self, context: ConnectionContext, ticket: Ticket, why: Problem) -> Answer {
        Answer::refused(context, Pending::new(&self, ticket), why)
    }
    /// Refuse without a connection (or on the browser's demo surface).
    #[must_use]
    pub fn refused(self, ticket: Ticket, why: Problem) -> Outcome {
        self.kind().refused(ticket, why)
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
        context: ConnectionContext,
        ticket: Ticket,
        ask: Ask,
    ) -> Result<(), Problem> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            enqueue(&self.commands, context, ticket, ask)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _request = (context, ticket, ask);
            Err(Problem::Refused(vessel_proto::ErrorCode::Unsupported))
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn enqueue(
    commands: &tokio::sync::mpsc::Sender<crate::Command>,
    context: ConnectionContext,
    ticket: Ticket,
    ask: Ask,
) -> Result<(), Problem> {
    crate::offer(
        commands,
        crate::Command::Devices {
            context,
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

/// Bounded work includes sending a completed reply to the UI.
#[cfg(not(target_arch = "wasm32"))]
const MAX_CALLS: usize = 8;
#[cfg(not(target_arch = "wasm32"))]
const CALL_DEADLINE: std::time::Duration = std::time::Duration::from_secs(15);

/// Calls are bounded even while a person switches machines repeatedly.
/// A stalled node cannot accumulate one background task per navigation.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub(crate) struct Hub(std::sync::Arc<tokio::sync::Semaphore>);

#[cfg(not(target_arch = "wasm32"))]
impl Default for Hub {
    fn default() -> Self {
        Self(std::sync::Arc::new(tokio::sync::Semaphore::new(MAX_CALLS)))
    }
}

// Explicit fixtures wait for the existing permits after closing their event
// receiver. No second task registry or counter is invented for tests.
#[cfg(all(not(target_arch = "wasm32"), any(test, feature = "testing")))]
impl Hub {
    pub(crate) async fn drained(&self) -> Result<(), tokio::sync::AcquireError> {
        let mut held = Vec::with_capacity(MAX_CALLS);
        for _ in 0..MAX_CALLS {
            held.push(self.0.acquire().await?);
        }
        drop(held);
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    pub(crate) async fn devices(&self, context: &ConnectionContext, ticket: Ticket, ask: Ask) {
        let pending = Pending::new(&ask, ticket);
        if self.onboarding_scope.check_control(context).is_err() {
            self.emit(crate::Event::Devices(Answer::refused(
                context.clone(),
                pending,
                receipt::problem(vessel_app_view::connection::Refusal::Changed),
            )))
            .await;
            return;
        }
        let connected = match self.selected_connection(context) {
            Ok(connected) => connected,
            Err(why) => {
                self.emit(crate::Event::Devices(Answer::refused(
                    context.clone(),
                    pending,
                    receipt::problem(why),
                )))
                .await;
                return;
            }
        };
        let Ok(permit) = std::sync::Arc::clone(&self.device_calls.0).try_acquire_owned() else {
            self.emit(crate::Event::Devices(Answer::refused(
                context.clone(),
                pending,
                Problem::Refused(vessel_proto::ErrorCode::Unavailable),
            )))
            .await;
            return;
        };
        let context = context.clone();
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let guard = connected.delivery_guard();
            let outcome = bounded_call(&connected, ticket, ask).await;
            let answer = Answer::completed(context.clone(), pending, guard, outcome);
            let sent = connected
                .send_guarded(&events, crate::Event::Devices(answer))
                .await;
            // Fallback egress holds only codes/correlation, never an old client.
            drop(connected);
            match sent {
                Ok(()) | Err(crate::connections::SendFailure::ReceiverClosed) => {}
                Err(crate::connections::SendFailure::Refused(why)) => {
                    let _closed = events
                        .send(crate::Event::Devices(Answer::retired(
                            context,
                            pending,
                            receipt::problem(why),
                        )))
                        .await;
                }
            }
            drop(permit);
        }));
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn bounded_call(
    client: &crate::connections::SelectedConnection,
    ticket: Ticket,
    ask: Ask,
) -> Outcome {
    // Keep the operation kind to answer a timeout without retaining a
    // credential or cloning a request. A timed-out write may have taken
    // effect; preserve that uncertainty rather than call it a refusal.
    let timed_out = ask.kind().timed_out(ticket);
    tokio::time::timeout(CALL_DEADLINE, call(client, ticket, ask))
        .await
        .unwrap_or(timed_out)
}

#[cfg(not(target_arch = "wasm32"))]
async fn call(
    client: &crate::connections::SelectedConnection,
    ticket: Ticket,
    ask: Ask,
) -> Outcome {
    match ask {
        Ask::Authority => Outcome::Authority(
            ticket,
            client
                .call(vessel_proto::GetDeviceSelf {})
                .await
                .map_err(selected_problem),
        ),
        Ask::List => Outcome::Listed(
            ticket,
            client
                .call(vessel_proto::ListDevices {})
                .await
                .map_err(selected_problem),
        ),
        Ask::Revoke { device } => Outcome::Revoked(
            ticket,
            client
                .call(vessel_proto::RevokeDevice { device })
                .await
                .map(|_ack| ())
                .map_err(selected_problem),
        ),
        Ask::Invite { label } => {
            let result = match client
                .call(vessel_proto::InviteDevice {
                    label,
                    ttl_secs: vessel_app_view::devices::LINK_TTL_SECS,
                    scopes: None,
                })
                .await
            {
                Ok(invitation) => match invitation.link.decode() {
                    Ok(offer) => vessel_app_view::devices::invitation_route(&offer)
                        .map(|()| Pairing::new(invitation, vessel_client::fingerprint(&offer.key))),
                    Err(_invalid) => Err(Problem::InvalidReply),
                },
                Err(error) => Err(selected_problem(error)),
            };
            Outcome::Invited(ticket, result)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn selected_problem(error: crate::connections::RequestFailure) -> Problem {
    match error {
        crate::connections::RequestFailure::Refused(
            vessel_app_view::connection::Refusal::Changed,
        ) => Problem::TargetChanged,
        crate::connections::RequestFailure::Refused(
            vessel_app_view::connection::Refusal::Closed,
        ) => Problem::Offline,
        crate::connections::RequestFailure::Call(error) => problem(error),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn problem(error: vessel_client::CallError) -> Problem {
    match error {
        vessel_client::CallError::Disconnected => Problem::Offline,
        vessel_client::CallError::Remote(error) => {
            if error.code == vessel_proto::ErrorCode::Conflict
                && matches!(
                    error.detail,
                    Some(vessel_proto::ErrorDetail::DeviceInvite {
                        reason: vessel_proto::DeviceInviteError::NoRoutes
                    })
                )
            {
                Problem::NoRoute
            } else {
                Problem::Refused(error.code)
            }
        }
        vessel_client::CallError::NotInProtocol { .. }
        | vessel_client::CallError::UnassignedProtocol { .. } => {
            Problem::Refused(vessel_proto::ErrorCode::Unsupported)
        }
        vessel_client::CallError::UnexpectedReply(_) => Problem::InvalidReply,
        vessel_client::CallError::RequestIdsExhausted => Problem::Exhausted,
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests;
