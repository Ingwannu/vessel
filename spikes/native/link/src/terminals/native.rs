//! The terminal calls on the desktop's runtime (see the parent module).

use std::collections::HashMap;
use std::time::Duration;

use bytes::Bytes;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use vessel_app_view::terminal::Outgoing;
use vessel_app_view::terminal_tabs::Failure;
use vessel_client::{CallError, NodeClient, StreamError};
use vessel_proto::terminals::{CloseTerminal, CreateTerminal, ListTerminals, SendTerminal};
use vessel_proto::{EventSeq, TerminalBytes, TerminalId, TerminalInput, TerminalSize};

use super::{Answer, Ask, Reply, Request};
use crate::Event;

/// The longest a unary terminal call is waited for. A send is small and
/// a create starts a shell, so this is generous; the node's own long
/// polls are not used here.
const CALL_DEADLINE: Duration = Duration::from_secs(15);

/// The follows running, one per machine and terminal.
#[derive(Debug, Default)]
pub(crate) struct Hub {
    follows: HashMap<(String, TerminalId), JoinHandle<()>>,
}

impl Hub {
    fn stop(&mut self, machine: &str, terminal: TerminalId) {
        if let Some(follow) = self.follows.remove(&(machine.to_owned(), terminal)) {
            follow.abort();
        }
    }

    /// Forget follows that ended by themselves, so the map holds only
    /// running ones (one per open tab).
    fn prune(&mut self) {
        self.follows.retain(|_, follow| !follow.is_finished());
    }
}

impl crate::State {
    /// Run `ask` on a task of its own; its answers come back as events.
    pub(crate) async fn terminals(&mut self, ask: Ask) {
        self.terminal_hub.prune();
        if let Request::Unfollow { terminal } = ask.request {
            self.terminal_hub.stop(&ask.machine, terminal);
            return;
        }
        if let Request::Close { terminal } | Request::Follow { terminal, .. } = ask.request {
            self.terminal_hub.stop(&ask.machine, terminal);
        }
        let Ok(client) = self.client(&ask.machine).cloned() else {
            if let Some(answer) = ask.refused(Failure::Offline) {
                self.emit(Event::Terminals(answer)).await;
            }
            return;
        };
        let events = self.events.clone();
        let machine = ask.machine;
        match ask.request {
            Request::Follow { terminal, from } => {
                let key = (machine.clone(), terminal);
                let task = tokio::spawn(follow(client, machine, terminal, from, events));
                drop(self.terminal_hub.follows.insert(key, task));
            }
            request => drop(tokio::spawn(async move {
                let reply = call(&client, request).await;
                let answer = Answer { machine, reply };
                let _closed = events.send(Event::Terminals(answer)).await;
            })),
        }
    }
}

/// One unary call, bounded, as its reply.
async fn call(client: &NodeClient, request: Request) -> Reply {
    match request {
        Request::Create { place } => {
            let asked = client.call(CreateTerminal {
                place: place.clone(),
                size: None,
                title: None,
            });
            Reply::Created {
                place,
                result: bounded(asked).await.map(Box::new),
            }
        }
        Request::List { place } => {
            let asked = client.call(ListTerminals {
                place: Some(place.clone()),
            });
            Reply::Listed {
                place,
                result: bounded(asked).await,
            }
        }
        Request::Send { terminal, outgoing } => {
            let input = match outgoing {
                Outgoing::Keys(text) => TerminalInput::Data {
                    data: TerminalBytes::new(Bytes::from(text.into_bytes())),
                },
                Outgoing::Resize { cols, rows } => TerminalInput::Resize {
                    size: TerminalSize { cols, rows },
                },
            };
            Reply::Sent {
                terminal,
                result: bounded(client.call(SendTerminal { terminal, input }))
                    .await
                    .map(drop),
            }
        }
        Request::Close { terminal } => Reply::Closed {
            terminal,
            result: bounded(client.call(CloseTerminal { terminal }))
                .await
                .map(drop),
        },
        // Taken before a task is started (`State::terminals`).
        Request::Follow { terminal, .. } | Request::Unfollow { terminal } => Reply::Lost {
            terminal,
            failure: Failure::Refused(vessel_proto::ErrorCode::InvalidRequest),
        },
    }
}

async fn bounded<T>(asked: impl Future<Output = Result<T, CallError>>) -> Result<T, Failure> {
    tokio::time::timeout(CALL_DEADLINE, asked)
        .await
        .unwrap_or(Err(CallError::Disconnected))
        .map_err(failure)
}

/// A terminal's records to the events channel, one event per record (the
/// screen folds whatever has queued up in one update), until it ends.
async fn follow(
    client: NodeClient,
    machine: String,
    terminal: TerminalId,
    from: EventSeq,
    events: mpsc::Sender<Event>,
) {
    let send = |reply: Reply| {
        let events = events.clone();
        let machine = machine.clone();
        async move {
            events
                .send(Event::Terminals(Answer { machine, reply }))
                .await
        }
    };
    let mut stream = match client.follow_terminal(terminal, from).await {
        Ok(stream) => stream,
        Err(error) => {
            let _closed = send(Reply::Lost {
                terminal,
                failure: failure(error),
            })
            .await;
            return;
        }
    };
    if send(Reply::Following(Box::new(stream.info().clone())))
        .await
        .is_err()
    {
        return;
    }
    loop {
        let reply = match stream.next().await {
            Some(Ok(record)) => Reply::Record { terminal, record },
            // Pruned scrollback: the screen goes on without it.
            Some(Err(StreamError::Missing { .. })) => continue,
            Some(Err(error)) => Reply::Lost {
                terminal,
                failure: stream_failure(error),
            },
            None => Reply::Ended { terminal },
        };
        let last = !matches!(reply, Reply::Record { .. });
        if send(reply).await.is_err() || last {
            return;
        }
    }
}

/// A call's failure as a code (never the node's text).
fn failure(error: CallError) -> Failure {
    match error {
        CallError::Disconnected => Failure::Offline,
        CallError::Remote(error) => Failure::Refused(error.code),
        CallError::NotInProtocol { .. } | CallError::UnassignedProtocol { .. } => {
            Failure::Refused(vessel_proto::ErrorCode::Unsupported)
        }
        CallError::UnexpectedReply(_) | CallError::RequestIdsExhausted => {
            Failure::Refused(vessel_proto::ErrorCode::Internal)
        }
    }
}

fn stream_failure(error: StreamError) -> Failure {
    match error {
        StreamError::Call(error) => failure(error),
        StreamError::Missing { .. }
        | StreamError::Unsubscribed
        | StreamError::Reconnecting { .. }
        | StreamError::Resumed { .. }
        | StreamError::Unreachable { .. } => Failure::Offline,
    }
}

#[cfg(test)]
mod tests {
    use super::{failure, stream_failure};
    use vessel_app_view::terminal_tabs::Failure;
    use vessel_client::{CallError, StreamError};
    use vessel_proto::{ErrorCode, WireError};

    #[test]
    fn a_refusal_crosses_as_its_code_only() {
        let raw = "a node's own words that may not reach the screen";
        let safe = failure(CallError::Remote(WireError::new(ErrorCode::NotFound, raw)));
        assert_eq!(safe, Failure::Refused(ErrorCode::NotFound));
        assert!(!format!("{safe:?}").contains(raw));
        assert_eq!(failure(CallError::Disconnected), Failure::Offline);
        assert_eq!(stream_failure(StreamError::Unsubscribed), Failure::Offline);
    }
}
