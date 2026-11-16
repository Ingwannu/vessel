//! Only the actor mints answers; the original weak lease is checked at receipt.
use super::{Answer, HostResult, Problem};
use vessel_app_view::connection::{ConnectionContext, Refusal};
use vessel_app_view::devices::Ticket;

/// A receipt checked immediately before installation by the consuming area.
/// Retain the weak guard with a settled report and recheck before copying.
#[derive(Debug)]
pub struct Received {
    /// The original selected physical admission.
    pub context: ConnectionContext,
    /// Original diagnostic run.
    pub ticket: Ticket,
    /// Only safe report data or a closed refusal; never a raw client error.
    pub result: Result<HostResult, Problem>,
    /// Weak validity, without ownership of the physical client.
    pub guard: Option<crate::DeliveryGuard>,
}
impl Answer {
    /// Check target/ticket before consuming an answer from the UI queue.
    #[must_use]
    pub const fn context(&self) -> &ConnectionContext {
        &self.context
    }
    /// Correlation is not an authority token.
    #[must_use]
    pub const fn ticket(&self) -> Ticket {
        self.ticket
    }
    /// Recheck the original admission after queueing, before any data installation.
    #[must_use]
    pub fn receive(self) -> Received {
        let (result, guard) = match self.guard {
            Some(guard) => match guard.validate(&self.context) {
                Ok(()) => (self.result, Some(guard)),
                Err(Refusal::Changed) => (Err(Problem::ConnectionChanged), None),
                Err(Refusal::Closed) => (Err(Problem::Offline), None),
            },
            None => (
                match self.result {
                    Ok(_) => Err(Problem::InvalidReply),
                    Err(why) => Err(why),
                },
                None,
            ),
        };
        Received {
            context: self.context,
            ticket: self.ticket,
            result,
            guard,
        }
    }
}
