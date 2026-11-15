//! Bounded metadata reads over the existing client; view owns paging and stale replies.
//! No transcript, provider or harness is touched by this adapter.

use vessel_app_view::prompt_index::{IndexAsk, IndexProblem};
use vessel_proto::prompt_index::PromptIndexPage;

/// One correlated metadata page.
#[derive(Debug)]
pub struct IndexAnswer {
    /// Exact ask, including target, page cursor and set identity.
    pub ask: IndexAsk,
    /// Safe typed result; raw peer error text is never retained.
    pub result: Result<PromptIndexPage, IndexProblem>,
}

/// Settle an app queue refusal through the same reply path as a node refusal.
#[must_use]
pub const fn rejected(why: crate::Rejected) -> IndexProblem {
    match why {
        crate::Rejected::Full => IndexProblem::Busy,
        crate::Rejected::Closed => IndexProblem::Connection,
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::{IndexAnswer, IndexAsk, IndexProblem};
    use std::{sync::Arc, time::Duration};
    use tokio::sync::{Semaphore, mpsc};
    use vessel_client::{CallError, NodeClient};
    use vessel_proto::ErrorCode;

    /// Covers the store's two admitted scans and the blocked event send too.
    const MAX_CALLS: usize = 2;
    /// Bound connection churn and a slow snapshot without blocking the link actor.
    const DEADLINE: Duration = Duration::from_secs(15);

    #[derive(Debug)]
    pub(crate) struct Hub(Arc<Semaphore>);

    impl Default for Hub {
        fn default() -> Self {
            Self(Arc::new(Semaphore::new(MAX_CALLS)))
        }
    }

    impl Hub {
        pub(crate) async fn ask(
            &self,
            client: Option<NodeClient>,
            ask: IndexAsk,
            events: mpsc::Sender<crate::Event>,
        ) {
            let Some(client) = client else {
                let _closed = events
                    .send(crate::Event::PromptIndex(IndexAnswer {
                        ask,
                        result: Err(IndexProblem::Connection),
                    }))
                    .await;
                return;
            };
            if ask.target.node.is_some_and(|node| client.node().id != node) {
                let _closed = events
                    .send(crate::Event::PromptIndex(IndexAnswer {
                        ask,
                        result: Err(IndexProblem::Connection),
                    }))
                    .await;
                return;
            }
            let Ok(permit) = Arc::clone(&self.0).try_acquire_owned() else {
                let _closed = events
                    .send(crate::Event::PromptIndex(IndexAnswer {
                        ask,
                        result: Err(IndexProblem::Busy),
                    }))
                    .await;
                return;
            };
            drop(tokio::spawn(async move {
                let result =
                    match tokio::time::timeout(DEADLINE, client.call(ask.request.clone())).await {
                        Ok(result) => result.map_err(|error| problem(&error)),
                        Err(_elapsed) => Err(IndexProblem::Timeout),
                    };
                let _closed = events
                    .send(crate::Event::PromptIndex(IndexAnswer { ask, result }))
                    .await;
                drop(permit);
            }));
        }
    }

    fn problem(error: &CallError) -> IndexProblem {
        if matches!(
            error,
            CallError::NotInProtocol { .. } | CallError::UnassignedProtocol { .. }
        ) {
            return IndexProblem::Unsupported;
        }
        match error.code() {
            Some(ErrorCode::Conflict) => IndexProblem::HistoryChanged,
            Some(ErrorCode::Unavailable) => IndexProblem::Busy,
            Some(code) => IndexProblem::Refused(code),
            None => IndexProblem::Connection,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::Hub;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
