//! Typed owner effects shared by host settings and deferred onboarding.
//!
//! [Decision Log]
//! - Reuse the real node calls; no app preference duplicates host configuration.
//! - Fixed node identity is checked before effects. One deadline bounds the
//!   typed node call; the admission permit also covers answer backpressure.
//! - Feature-pending and old nodes refuse without calling the RPC. A timeout
//!   means unconfirmed, because the node retains admitted writes after cancellation.
pub use vessel_app_view::orchestration::{Action, Answer, Output, Problem, Task};

#[cfg(not(target_arch = "wasm32"))]
mod calls;
#[cfg(not(target_arch = "wasm32"))]
mod runner;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

/// A rejected queue settles exactly the originating task.
#[must_use]
pub const fn rejected(why: crate::Rejected) -> Problem {
    match why {
        crate::Rejected::Full => Problem::Busy,
        crate::Rejected::Closed => Problem::Offline,
    }
}
#[cfg(not(target_arch = "wasm32"))]
const MAX_TASKS: usize = 8;
/// Admission capacity includes pending result sends; the RPC timer does not.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub(crate) struct Hub(std::sync::Arc<tokio::sync::Semaphore>);
#[cfg(not(target_arch = "wasm32"))]
impl Default for Hub {
    fn default() -> Self {
        Self(std::sync::Arc::new(tokio::sync::Semaphore::new(MAX_TASKS)))
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    pub(crate) async fn orchestration(&self, task: Task) {
        let Ok(connection) = self.selected_connection(&task.scope.target) else {
            self.emit(crate::Event::Orchestration(task.refused(Problem::Changed)))
                .await;
            return;
        };
        let Ok(permit) = std::sync::Arc::clone(&self.orchestration_calls.0).try_acquire_owned()
        else {
            self.emit(crate::Event::Orchestration(task.refused(Problem::Busy)))
                .await;
            return;
        };
        let client = calls::Selected {
            connection,
            context: task.scope.target.clone(),
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let _permit = permit;
            let result = runner::run(&client, &task).await;
            let _sent = events
                .send(crate::Event::Orchestration(Answer { task, result }))
                .await;
        }));
    }
}
