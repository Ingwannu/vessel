//! One immutable selected client, bounded readonly work, guarded queue delivery.
use super::{Answer, Body, Problem, Request};
use crate::{
    Event, State,
    connections::{RequestFailure, SelectedConnection, SendFailure},
};
use std::sync::Arc;
use tokio::sync::Semaphore;
use vessel_app_view::connection::Refusal;
use vessel_client::{CallError, Unary};

const MAX_READS: usize = 8;
const READ_DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);
#[derive(Debug, Clone)]
pub(crate) struct Hub(Arc<Semaphore>);
impl Default for Hub {
    fn default() -> Self {
        Self(Arc::new(Semaphore::new(MAX_READS)))
    }
}
impl State {
    pub(crate) async fn code_source(&self, request: Request) {
        let selected = match self.selected_connection(request.context()) {
            Ok(selected) => selected,
            Err(why) => {
                self.emit(Event::CodeSource(Box::new(request.refused(refusal(why)))))
                    .await;
                return;
            }
        };
        let Ok(permit) = Arc::clone(&self.workspace.source.0).try_acquire_owned() else {
            self.emit(Event::CodeSource(Box::new(
                request.refused(Problem::Unsent),
            )))
            .await;
            return;
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let guard = selected.delivery_guard();
            let body = body(&selected, &request).await;
            let fallback = request.clone();
            let answer = Answer {
                request,
                guard: Some(guard),
                body,
            };
            let sent = selected
                .send_guarded(&events, Event::CodeSource(Box::new(answer)))
                .await;
            drop(selected);
            match sent {
                Ok(()) | Err(SendFailure::ReceiverClosed) => {}
                Err(SendFailure::Refused(why)) => {
                    let _closed = events
                        .send(Event::CodeSource(Box::new(fallback.refused(refusal(why)))))
                        .await;
                }
            }
            drop(permit);
        }));
    }
}
async fn read<R: Unary>(selected: &SelectedConnection, request: R) -> Result<R::Response, Problem> {
    tokio::time::timeout(READ_DEADLINE, selected.call(request))
        .await
        .map_err(|_elapsed| Problem::Gone(String::new()))?
        .map_err(problem)
}
async fn body(selected: &SelectedConnection, request: &Request) -> Body {
    match request {
        Request::Root { session, .. } => Body::Root(
            read(selected, vessel_proto::GetSession { session: *session })
                .await
                .and_then(|info| {
                    if info.id == *session {
                        Ok(Box::new(info))
                    } else {
                        Err(Problem::Unsent)
                    }
                }),
        ),
        Request::Summary { ask, scope, .. } => Body::Summary(if ask.root == scope.root() {
            read(selected, ask.clone()).await
        } else {
            Err(Problem::Unsent)
        }),
        Request::Diff { ask, scope, .. } => Body::Diff(if ask.root == scope.root() {
            read(selected, ask.clone()).await.and_then(|page| {
                if page.path == ask.path {
                    Ok(page)
                } else {
                    Err(Problem::Unsent)
                }
            })
        } else {
            Err(Problem::Unsent)
        }),
    }
}
fn refusal(_why: Refusal) -> Problem {
    Problem::Gone(String::new())
}
fn problem(error: RequestFailure) -> Problem {
    match error {
        RequestFailure::Refused(why) => refusal(why),
        RequestFailure::Call(CallError::Remote(error)) => Problem::Node {
            code: error.code,
            message: String::new(),
        },
        RequestFailure::Call(
            CallError::NotInProtocol { .. } | CallError::UnassignedProtocol { .. },
        ) => Problem::Older,
        RequestFailure::Call(CallError::RequestIdsExhausted | CallError::UnexpectedReply(_)) => {
            Problem::Unsent
        }
        RequestFailure::Call(CallError::Disconnected) => Problem::Gone(String::new()),
    }
}
