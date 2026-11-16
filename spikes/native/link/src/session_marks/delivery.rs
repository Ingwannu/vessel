//! No successful directory payload can bypass original queued-delivery validity.
use super::{Ask, Failure, Reply};

/// Either a guarded observation or a correlated control-only refusal.
#[derive(Debug)]
pub struct Answer {
    reply: Reply,
    guard: Option<crate::DeliveryGuard>,
}
impl Answer {
    /// No fake guard or successful data is created for local queue/platform refusal.
    #[must_use]
    pub fn refused(ask: Ask, failure: Failure) -> Self {
        Self {
            reply: Reply {
                ask,
                result: Err(failure),
            },
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
    /// Validate immediately before matching/consuming pending UI and applying data.
    /// A dispatched result invalidated after queueing remains uncertain.
    #[must_use]
    pub fn into_reply(mut self) -> Reply {
        if let Some(guard) = self.guard
            && guard.validate(self.reply.ask.target().context()).is_err()
        {
            self.reply.result = Err(Failure::Uncertain);
        }
        self.reply
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn publish(
    selected: &crate::connections::SelectedConnection,
    events: &tokio::sync::mpsc::Sender<crate::Event>,
    reply: Reply,
    guard: crate::DeliveryGuard,
) {
    use crate::connections::SendFailure;
    let ask = reply.ask.clone();
    let event = crate::Event::SessionMarked(Answer::observed(reply, guard));
    match tokio::time::timeout(
        super::native::PUBLICATION_DEADLINE,
        selected.send_guarded(events, event),
    )
    .await
    {
        Ok(Ok(()) | Err(SendFailure::ReceiverClosed)) => return,
        Ok(Err(SendFailure::Refused(_))) | Err(_) => {}
    }
    // Capacity stays held until the original live owner can settle uncertainty;
    // dropping a second timeout could strand an already committed operation.
    let _closed = events
        .send(crate::Event::SessionMarked(Answer::refused(
            ask,
            Failure::Uncertain,
        )))
        .await;
}
