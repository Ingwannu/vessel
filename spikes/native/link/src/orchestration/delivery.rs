//! Result envelopes own data and weak validity, never an original client.
use super::{Problem, Task};
use vessel_app_view::connection::Refusal;
use vessel_app_view::orchestration::Answer as Reply;

/// A correlated local refusal or guarded node observation, with private payload.
/// Onboarding retains this envelope until its own final installation point.
#[derive(Debug)]
pub struct Answer {
    reply: Reply,
    guard: Option<crate::DeliveryGuard>,
}
impl Answer {
    /// Construct only a refusal for a request that was not admitted.
    #[must_use]
    pub fn refused(task: Task, problem: Problem) -> Self {
        Self {
            reply: task.refused(problem),
            guard: None,
        }
    }
    /// Route by original correlation without exposing an unvalidated result.
    #[must_use]
    pub const fn task(&self) -> &Task {
        &self.reply.task
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn observed(reply: Reply, guard: crate::DeliveryGuard) -> Self {
        Self {
            reply,
            guard: Some(guard),
        }
    }
    /// Validate immediately before the model applies this result, without an
    /// await between them. The model also matches full scope and attempt.
    #[must_use]
    pub fn into_reply(self) -> Reply {
        if let Some(guard) = self.guard
            && let Err(refusal) = guard.validate(&self.reply.task.scope.target)
        {
            let problem = invalidated(&self.reply.task, refusal);
            return self.reply.task.refused(problem);
        }
        self.reply
    }
}

fn invalidated(task: &Task, refusal: Refusal) -> Problem {
    if task.action.changes_node() {
        return Problem::Uncertain;
    }
    match refusal {
        Refusal::Changed => Problem::Changed,
        Refusal::Closed => Problem::Offline,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn publish(
    selected: &crate::connections::SelectedConnection,
    events: &tokio::sync::mpsc::Sender<crate::Event>,
    task: Task,
    result: Result<super::Output, Problem>,
    guard: crate::DeliveryGuard,
) {
    use crate::connections::SendFailure;
    let observed = Answer::observed(
        Reply {
            task: task.clone(),
            result,
        },
        guard,
    );
    let sent = tokio::time::timeout(
        super::PUBLICATION_BUDGET,
        selected.send_guarded(events, crate::Event::Orchestration(observed)),
    )
    .await;
    let problem = match sent {
        Ok(Ok(()) | Err(SendFailure::ReceiverClosed)) => return,
        Ok(Err(SendFailure::Refused(refusal))) => invalidated(&task, refusal),
        Err(_elapsed) if task.action.changes_node() => Problem::Uncertain,
        Err(_elapsed) => Problem::TimedOut,
    };
    // Retain bounded admission until the exact ticket can settle. A second
    // dropping timeout strands admitted writes. Receiver close releases this
    // wait; it sends no effect retry and creates no extra queue.
    let _closed = events
        .send(crate::Event::Orchestration(Answer::refused(task, problem)))
        .await;
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "delivery_tests.rs"]
mod tests;
