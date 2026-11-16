//! Bounded metadata reads over the existing client; view owns paging and stale replies.
//! No transcript, provider or harness is touched by this adapter.

use vessel_app_view::prompt_index::{IndexAsk, IndexProblem};
use vessel_proto::prompt_index::PromptIndexPage;

/// One correlated metadata page.
#[derive(Debug)]
pub struct IndexAnswer {
    /// Exact ask, including target, page cursor and set identity.
    pub ask: IndexAsk,
    /// Weak validity of the original lease, never a transport owner.
    pub delivery: Option<crate::DeliveryGuard>,
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
    use crate::connections::{RequestFailure, SelectedConnection};
    use std::{sync::Arc, time::Duration};
    use tokio::sync::{Semaphore, mpsc};
    use vessel_app_view::connection::Refusal;
    use vessel_client::CallError;
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
            client: Result<SelectedConnection, Refusal>,
            ask: IndexAsk,
            events: mpsc::Sender<crate::Event>,
        ) {
            let Ok(client) = client else {
                let _closed = events
                    .send(crate::Event::PromptIndex(IndexAnswer {
                        ask,
                        delivery: None,
                        result: Err(IndexProblem::Connection),
                    }))
                    .await;
                return;
            };
            let Ok(permit) = Arc::clone(&self.0).try_acquire_owned() else {
                // A busy refusal has no selected work; queue pressure must not
                // retain the rejected connection while its notice waits.
                drop(client);
                let _closed = events
                    .send(crate::Event::PromptIndex(IndexAnswer {
                        ask,
                        delivery: None,
                        result: Err(IndexProblem::Busy),
                    }))
                    .await;
                return;
            };
            drop(tokio::spawn(async move {
                let delivery = client.delivery_guard();
                let fallback = ask.clone();
                let result =
                    match tokio::time::timeout(DEADLINE, client.call(ask.request.clone())).await {
                        Ok(result) => result.map_err(|error| problem(&error)),
                        Err(_elapsed) => Err(IndexProblem::Timeout),
                    };
                let event = crate::Event::PromptIndex(IndexAnswer {
                    ask,
                    delivery: Some(delivery),
                    result,
                });
                if client.send_guarded(&events, event).await.is_err() {
                    drop(client);
                    let _closed = events
                        .send(crate::Event::PromptIndex(IndexAnswer {
                            ask: fallback,
                            delivery: None,
                            result: Err(IndexProblem::Connection),
                        }))
                        .await;
                }
                drop(permit);
            }));
        }
    }

    impl crate::State {
        pub(crate) async fn prompt_index(&self, ask: IndexAsk) {
            let client = ask
                .target
                .connection
                .as_ref()
                .filter(|context| context.machine() == &ask.target.machine)
                .ok_or(Refusal::Changed)
                .and_then(|context| self.selected_connection(context));
            self.prompt_calls
                .ask(client, ask, self.events.clone())
                .await;
        }
    }

    fn problem(error: &RequestFailure) -> IndexProblem {
        match error {
            RequestFailure::Refused(Refusal::Changed | Refusal::Closed) => IndexProblem::Connection,
            RequestFailure::Call(error) => call_problem(error),
        }
    }

    fn call_problem(error: &CallError) -> IndexProblem {
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

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests;
