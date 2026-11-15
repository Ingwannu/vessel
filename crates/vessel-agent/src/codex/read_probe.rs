//! Shared prompt-free transport extracted from W22's catalogue transaction.
//! Policy and typed native response projection remain with each read operation.
use super::protocol::{ClientInfo, InitializeCapabilities, InitializeParams, InitializeResponse};
use super::wire::{self, Incoming, Outgoing};
use std::{future::Future, time::Duration};
use tokio::{sync::mpsc, task::JoinSet};
use vessel_proc::{CommandSpec, Signal};

const INCOMING: usize = 16;
const MAX_NOTIFICATIONS: usize = 256;
const STOP_GRACE: Duration = Duration::from_secs(2);
const METHOD_NOT_FOUND: i64 = -32601;

/// Safe failure classes; native error/log text never crosses into diagnostics.
#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    /// The finite read deadline elapsed.
    #[error("Codex read-only probe deadline elapsed")]
    TimedOut,
    /// Process setup failed; the source stays internal to the node.
    #[error("Codex read-only probe process could not start")]
    Spawn(#[source] vessel_proc::SpawnError),
    /// The connection or a required request failed.
    #[error("Codex read-only probe protocol failed")]
    Protocol,
    /// A native response violated its bounded typed contract.
    #[error("Codex read-only observation was invalid")]
    Invalid,
}
/// Implemented only by reviewed prompt-free native read requests inside this driver.
pub(super) trait ReadRequest: serde::Serialize {
    type Response: serde::de::DeserializeOwned;
    const METHOD: &'static str;
}
impl ReadRequest for InitializeParams {
    type Response = InitializeResponse;
    const METHOD: &'static str = "initialize";
}
/// One original child process and its bounded readers, never a second parser.
pub(super) async fn run<T, F, Fut>(
    command: &CommandSpec,
    limit: Duration,
    read: F,
) -> Result<T, ProbeError>
where
    F: FnOnce(Transaction) -> Fut,
    Fut: Future<Output = Result<T, ProbeError>>,
{
    let vessel_proc::Spawned {
        mut child,
        stdin,
        stdout,
        mut stderr,
    } = vessel_proc::spawn(command).map_err(ProbeError::Spawn)?;
    let out = Outgoing::start(stdin);
    let (send, incoming) = mpsc::channel(INCOMING);
    // Cancellation drops JoinSet and the existing process-group guard together.
    let mut readers = JoinSet::new();
    let _stdout_task = readers.spawn(wire::read(stdout, send));
    let _stderr_task = readers.spawn(async move {
        let _discarded = tokio::io::copy(&mut stderr, &mut tokio::io::sink()).await;
    });
    let transaction = Transaction {
        out,
        incoming,
        next: 0,
    };
    let observed = tokio::time::timeout(limit, read(transaction))
        .await
        .map_err(|_elapsed| ProbeError::TimedOut)
        .and_then(std::convert::identity);
    let _terminated = child.signal(Signal::Terminate);
    if tokio::time::timeout(STOP_GRACE, child.wait())
        .await
        .is_err()
    {
        let _killed = child.signal(Signal::Kill);
        let _reaped = tokio::time::timeout(STOP_GRACE, child.wait()).await;
    }
    readers.shutdown().await;
    observed
}
pub(super) struct Transaction {
    out: Outgoing,
    incoming: mpsc::Receiver<Result<Incoming, wire::Ended>>,
    next: u64,
}
impl Transaction {
    pub(super) async fn initialize(&mut self) -> Result<InitializeResponse, ProbeError> {
        let initialize = InitializeParams {
            client_info: ClientInfo {
                name: "vessel",
                title: "Vessel",
                version: env!("CARGO_PKG_VERSION"),
            },
            capabilities: InitializeCapabilities {
                experimental_api: true,
            },
        };
        let initialized = self.required(&initialize).await?;
        if !self.out.notify("initialized", &serde_json::json!({})).await {
            return Err(ProbeError::Protocol);
        }
        Ok(initialized)
    }
    pub(super) async fn required<R: ReadRequest>(
        &mut self,
        request: &R,
    ) -> Result<R::Response, ProbeError> {
        self.request(request).await?.ok_or(ProbeError::Protocol)
    }
    /// Only method-not-found is optional. Invalid params/data are not empty success.
    pub(super) async fn request<R: ReadRequest>(
        &mut self,
        request: &R,
    ) -> Result<Option<R::Response>, ProbeError> {
        self.next = self.next.checked_add(1).ok_or(ProbeError::Protocol)?;
        if !self.out.request(self.next, R::METHOD, request).await {
            return Err(ProbeError::Protocol);
        }
        for _message in 0..MAX_NOTIFICATIONS {
            match self.incoming.recv().await {
                Some(Ok(Incoming::Response { id, result })) if id == self.next => {
                    return serde_json::from_value(result)
                        .map(Some)
                        .map_err(|_invalid| ProbeError::Invalid);
                }
                Some(Ok(Incoming::Failure { id, error }))
                    if id == self.next && error.code == METHOD_NOT_FOUND =>
                {
                    return Ok(None);
                }
                Some(Ok(Incoming::Notification { .. })) => {}
                // A read probe cannot ask for execution, tools, sign-in or human approval.
                Some(
                    Ok(
                        Incoming::Response { .. }
                        | Incoming::Failure { .. }
                        | Incoming::Request { .. },
                    )
                    | Err(_),
                )
                | None => return Err(ProbeError::Protocol),
            }
        }
        Err(ProbeError::Protocol)
    }
}

#[cfg(test)]
impl Transaction {
    pub(super) fn in_memory() -> (
        Self,
        mpsc::Receiver<Vec<u8>>,
        mpsc::Sender<Result<Incoming, wire::Ended>>,
    ) {
        let (out, requests) = Outgoing::detached();
        let (responses, incoming) = mpsc::channel(INCOMING);
        (
            Self {
                out,
                incoming,
                next: 0,
            },
            requests,
            responses,
        )
    }
}
