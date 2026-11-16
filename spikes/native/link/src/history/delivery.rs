//! Real queued replies retain validity without retaining the original transport.
use super::{Ask, Reply};
use vessel_app_view::history::Problem;
/// A result is either a correlated local refusal or an observed guarded payload.
/// Private fields prevent an unguarded successful payload from being constructed.
#[derive(Debug)]
pub struct Answer {
    reply: Reply,
    guard: Option<crate::DeliveryGuard>,
}
impl Answer {
    /// Settle a command that was not admitted, without constructing a fake guard
    /// or allowing an unguarded successful payload.
    #[must_use]
    pub fn refused(request: Ask, problem: Problem) -> Self {
        Self {
            reply: request.refused(problem),
            guard: None,
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn observed(reply: Reply, guard: crate::DeliveryGuard) -> Self {
        Self {
            reply,
            guard: Some(guard),
        }
    }
    /// Validate immediately before applying the model reply, with no intervening
    /// await. No invalidated data or successful mutation receipt escapes this edge.
    #[must_use]
    pub fn into_reply(self) -> Reply {
        if let Some(guard) = self.guard
            && let Err(refusal) = guard.validate(&self.reply.scope().context)
        {
            let problem = match refusal {
                vessel_app_view::connection::Refusal::Changed => Problem::IdentityChanged,
                vessel_app_view::connection::Refusal::Closed => Problem::Offline,
            };
            return self.reply.invalidated(problem);
        }
        self.reply
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn publish(
    selected: &crate::connections::SelectedConnection,
    events: &tokio::sync::mpsc::Sender<crate::Event>,
    request: Ask,
    reply: Reply,
    guard: crate::DeliveryGuard,
) {
    use crate::connections::SendFailure;
    use vessel_app_view::connection::Refusal;
    let event = crate::Event::History(Answer::observed(reply, guard));
    let published = tokio::time::timeout(
        super::PUBLICATION_DEADLINE,
        selected.send_guarded(events, event),
    )
    .await;
    let problem = match published {
        Ok(Ok(()) | Err(SendFailure::ReceiverClosed)) => return,
        Ok(Err(SendFailure::Refused(Refusal::Changed))) => Problem::IdentityChanged,
        Ok(Err(SendFailure::Refused(Refusal::Closed))) => Problem::Offline,
        Err(_deadline) => Problem::Timeout,
    };
    // This is a correlated refusal, never an unguarded data delivery. Writes
    // may already have reached the node; only an explicit fresh read reconciles.
    let problem = if matches!(&request, Ask::Import(_)) {
        Problem::Uncertain
    } else {
        problem
    };
    let fallback = crate::Event::History(Answer::refused(request, problem));
    // Retain the bounded call permit until the live owner can settle this ticket.
    // A second dropping timeout would strand an already committed import forever.
    // Receiver closure still releases the task; no retry or unbounded queue exists.
    let _closed = events.send(fallback).await;
}

#[cfg(test)]
#[path = "delivery_tests.rs"]
mod tests;
