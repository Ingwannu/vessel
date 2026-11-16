//! Queue admission and application are different moments. A weak guard prevents
//! a completed reply from installing after its original transport is retired.
//! Why: a Fleet notice may still be queued behind this result; context equality
//! alone cannot prove physical liveness at the exact application edge.
use super::{Message, Request, ScheduleError};
use crate::DeliveryGuard;

/// Observed payload; only the real actor can attach the original weak guard.
#[derive(Debug)]
pub struct Delivered {
    request: Request,
    message: Box<Message>,
    guard: DeliveryGuard,
}
impl Delivered {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn observed(request: Request, message: Message, guard: DeliveryGuard) -> Self {
        Self {
            request,
            message: Box::new(message),
            guard,
        }
    }
    /// Check immediately before model installation, without an intervening await.
    /// A retired write becomes uncertain; its original resource/ticket survives.
    #[must_use]
    pub fn settle(self) -> Message {
        match self.guard.validate(self.request.connection()) {
            Ok(()) => *self.message,
            Err(why) => self.request.refused(ScheduleError::Retired(why)),
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn publish(
    selected: crate::connections::SelectedConnection,
    events: &tokio::sync::mpsc::Sender<crate::Event>,
    request: Request,
    message: Message,
    guard: DeliveryGuard,
) {
    use crate::connections::SendFailure;
    let message = crate::Event::Schedules(Message::Delivered(Delivered::observed(
        request.clone(),
        message,
        guard,
    )));
    let sent = tokio::time::timeout(
        super::calls::REQUEST_DEADLINE,
        selected.send_guarded(events, message),
    )
    .await;
    let error = match sent {
        Ok(Ok(()) | Err(SendFailure::ReceiverClosed)) => return,
        Ok(Err(SendFailure::Refused(why))) => ScheduleError::Retired(why),
        Err(_deadline) => ScheduleError::Deadline,
    };
    // The single terminal refusal retains the existing permit until consumed.
    // Never drop a committed write's only receipt under a second timeout.
    // Drop strong transport ownership before waiting on that terminal channel.
    drop(selected);
    let _closed = events
        .send(crate::Event::Schedules(request.refused(error)))
        .await;
}
