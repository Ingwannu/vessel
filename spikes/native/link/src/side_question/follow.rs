//! One independently owned side subscription. Replacing or closing it
//! aborts only this local reader, never a session or the main stream.

use std::sync::Arc;

use tokio::sync::{Semaphore, mpsc};
use vessel_app_view::side_question::{Target, Ticket};
use vessel_client::{NodeClient, StreamError};
use vessel_proto::{EventSeq, SessionId};

use super::{Failure, Notice, Request, call};

#[derive(Debug)]
pub(crate) struct Hub {
    calls: Arc<Semaphore>,
    watching: Option<(Ticket, tokio::task::JoinHandle<()>)>,
}

impl Default for Hub {
    fn default() -> Self {
        Self {
            calls: Arc::new(Semaphore::new(call::CALLS)),
            watching: None,
        }
    }
}

impl Drop for Hub {
    fn drop(&mut self) {
        if let Some((_ticket, task)) = self.watching.take() {
            task.abort();
        }
    }
}

impl crate::State {
    pub(crate) async fn side_question(&mut self, request: Request) {
        match request {
            Request::Call {
                target,
                ticket,
                ask,
            } => {
                let client = self
                    .clients
                    .get(target.machine.as_str())
                    .map(|connected| connected.client.clone());
                call::dispatch(
                    &self.side_questions.calls,
                    client,
                    self.events.clone(),
                    target,
                    ticket,
                    ask,
                )
                .await;
            }
            Request::Watch {
                target,
                ticket,
                side,
                from,
            } => {
                if let Some((_old_ticket, task)) = self.side_questions.watching.take() {
                    task.abort();
                }
                let client = self
                    .clients
                    .get(target.machine.as_str())
                    .map(|connected| connected.client.clone());
                let task = tokio::spawn(follow(
                    client,
                    self.events.clone(),
                    target,
                    ticket,
                    side,
                    from,
                ));
                self.side_questions.watching = Some((ticket, task));
            }
            Request::Close(ticket) => {
                if self
                    .side_questions
                    .watching
                    .as_ref()
                    .is_some_and(|(current, _task)| *current == ticket)
                    && let Some((_ticket, task)) = self.side_questions.watching.take()
                {
                    task.abort();
                }
            }
        }
    }
}

async fn emit(events: &mpsc::Sender<crate::Event>, notice: Notice) -> bool {
    events
        .send(crate::Event::SideQuestion(notice))
        .await
        .is_ok()
}

async fn follow(
    client: Option<NodeClient>,
    events: mpsc::Sender<crate::Event>,
    target: Target,
    ticket: Ticket,
    side: SessionId,
    from: EventSeq,
) {
    let lost = |why| Notice::Lost {
        target: target.clone(),
        ticket,
        side,
        why,
    };
    let Some(client) = client else {
        let _closed = emit(&events, lost(Failure::Offline)).await;
        return;
    };
    let mut stream = match client.follow(side, from).await {
        Ok(stream) => stream,
        Err(error) => {
            let _closed = emit(&events, lost(call::failure(error))).await;
            return;
        }
    };
    while let Some(item) = stream.next().await {
        let notice = match item {
            Ok(record) => Notice::Record {
                target: target.clone(),
                ticket,
                side,
                record: Box::new(record),
            },
            Err(StreamError::Missing {
                expected,
                resumed_at,
            }) => Notice::Pruned {
                target: target.clone(),
                ticket,
                side,
                expected,
                resumed_at,
            },
            Err(StreamError::Resumed { .. }) => continue,
            Err(StreamError::Call(error)) => {
                let _closed = emit(&events, lost(call::failure(error))).await;
                return;
            }
            Err(
                StreamError::Unsubscribed
                | StreamError::Reconnecting { .. }
                | StreamError::Unreachable { .. },
            ) => {
                let _closed = emit(&events, lost(Failure::Offline)).await;
                return;
            }
        };
        if !emit(&events, notice).await {
            return;
        }
    }
    let _closed = emit(
        &events,
        Notice::Ended {
            target,
            ticket,
            side,
        },
    )
    .await;
}
