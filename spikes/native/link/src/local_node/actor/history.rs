//! Released direct-child observations survive a later authenticated reconnect.

use std::future::Future;
use tokio::task::JoinSet;

const MAX_RELEASED_OBSERVATIONS: usize = 8;

#[derive(Default)]
pub(super) struct History {
    observations: JoinSet<()>,
}

impl History {
    pub(super) fn prune(&mut self) {
        while let Some(result) = self.observations.try_join_next() {
            if result.is_err() {
                // Losing an observer is not proof that its child was reaped.
                self.retain(std::future::pending());
            }
        }
    }

    pub(super) fn has_capacity(&mut self) -> bool {
        self.prune();
        self.observations.len() < MAX_RELEASED_OBSERVATIONS
    }

    pub(super) fn retain(&mut self, observation: impl Future<Output = ()> + Send + 'static) {
        drop(self.observations.spawn(observation));
    }

    pub(super) async fn wait(&mut self) {
        while let Some(result) = self.observations.join_next().await {
            if result.is_err() {
                std::future::pending::<()>().await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::oneshot;

    #[tokio::test]
    async fn reconnect_status_cannot_release_an_earlier_child_obligation() {
        let mut history = History::default();
        let (exit, original_exit) = oneshot::channel();
        history.retain(async move {
            original_exit.await.unwrap();
        });
        {
            let waiting = history.wait();
            tokio::pin!(waiting);
            let pending = std::future::poll_fn(|cx| {
                std::task::Poll::Ready(waiting.as_mut().poll(cx).is_pending())
            })
            .await;
            assert!(pending);
            exit.send(()).unwrap();
            waiting.await;
        }
        assert!(history.has_capacity());
    }

    #[tokio::test]
    async fn unresolved_observers_bound_reconnect_admission() {
        let mut history = History::default();
        for _ in 0..MAX_RELEASED_OBSERVATIONS {
            assert!(history.has_capacity());
            history.retain(std::future::pending());
        }
        assert!(!history.has_capacity());
    }
}
