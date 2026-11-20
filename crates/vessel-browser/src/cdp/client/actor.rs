//! A single owner serializes bounded browser operations and retains shutdown.

use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use vessel_proc::OwnedGroupExit;
use vessel_proto::WorkspaceId;
use vessel_proto::browser::{
    BrowserEffect, BrowserFailure, BrowserId, BrowserLoad, BrowserRevision, BrowserUrl,
    COMMAND_TIMEOUT, HumanInput, ViewportSize,
};

use super::Client;
use super::owner::{Owner, OwnerError};
use crate::cdp::wire::{Event, EventKind};
use crate::delivery::{Attempt, Caller};
use crate::page::PageError;
use crate::targets::{TargetError, TargetWatch, Targets};

const COMMAND_CAPACITY: usize = 64;

#[derive(Debug, thiserror::Error)]
pub(super) enum ActorError {
    #[error("browser command queue is full")]
    Busy,
    #[error("browser actor is unavailable")]
    Unavailable,
    #[error("browser actor ended while an operation was pending ({effect:?})")]
    Disconnected {
        effect: BrowserEffect,
        #[source]
        cause: oneshot::error::RecvError,
    },
    #[error("browser operation exceeded its deadline ({effect:?})")]
    Timeout {
        effect: BrowserEffect,
        #[source]
        cause: tokio::time::error::Elapsed,
    },
    #[error("browser target operation failed")]
    Target(#[from] TargetError),
    #[error("browser page operation failed")]
    Page(#[from] PageError),
}

type Reply<T> = oneshot::Sender<Result<T, ActorError>>;

enum Command {
    Open(BrowserUrl, ViewportSize, Reply<BrowserId>),
    Watch(BrowserId, Reply<TargetWatch>),
    Input(BrowserId, BrowserRevision, HumanInput, Reply<()>),
    Navigate(BrowserId, BrowserUrl, Reply<()>),
    Close(BrowserId, Reply<()>),
}

struct Queued {
    workspace: WorkspaceId,
    attempt: Arc<Attempt>,
    command: Command,
}

/// Dropping the handle requests orderly shutdown while the task retains ownership.
pub(super) struct Actor {
    commands: mpsc::Sender<Queued>,
    stop: CancellationToken,
    task: JoinHandle<Shutdown>,
}

/// Keep the private profile held after the group receipt until the supervisor
/// decides whether it can be reused. The receipt alone grants no reuse.
pub(super) struct Shutdown {
    pub(super) receipt: Result<OwnedGroupExit, OwnerError>,
    _owner: Owner,
}

impl Actor {
    pub(super) fn start(owner: Owner) -> Self {
        let (commands, requests) = mpsc::channel(COMMAND_CAPACITY);
        let stop = CancellationToken::new();
        let task = tokio::spawn(run(owner, requests, stop.clone()));
        Self {
            commands,
            stop,
            task,
        }
    }

    pub(super) async fn open(
        &self,
        workspace: WorkspaceId,
        url: BrowserUrl,
        viewport: ViewportSize,
    ) -> Result<BrowserId, ActorError> {
        self.call(workspace, |reply| Command::Open(url, viewport, reply))
            .await
    }

    pub(super) async fn watch(
        &self,
        workspace: WorkspaceId,
        browser: BrowserId,
    ) -> Result<TargetWatch, ActorError> {
        self.call(workspace, |reply| Command::Watch(browser, reply))
            .await
    }

    pub(super) async fn input(
        &self,
        workspace: WorkspaceId,
        browser: BrowserId,
        revision: BrowserRevision,
        input: HumanInput,
    ) -> Result<(), ActorError> {
        self.call(workspace, |reply| {
            Command::Input(browser, revision, input, reply)
        })
        .await
    }

    pub(super) async fn navigate(
        &self,
        workspace: WorkspaceId,
        browser: BrowserId,
        url: BrowserUrl,
    ) -> Result<(), ActorError> {
        self.call(workspace, |reply| Command::Navigate(browser, url, reply))
            .await
    }

    pub(super) async fn close(
        &self,
        workspace: WorkspaceId,
        browser: BrowserId,
    ) -> Result<(), ActorError> {
        self.call(workspace, |reply| Command::Close(browser, reply))
            .await
    }

    pub(super) async fn shutdown(mut self) -> Result<Shutdown, tokio::task::JoinError> {
        self.stop.cancel();
        (&mut self.task).await
    }

    async fn call<T>(
        &self,
        workspace: WorkspaceId,
        make: impl FnOnce(Reply<T>) -> Command,
    ) -> Result<T, ActorError> {
        if self.stop.is_cancelled() {
            return Err(ActorError::Unavailable);
        }
        let attempt = Arc::new(Attempt::new());
        let caller = Caller::new(Arc::clone(&attempt));
        let (reply, result) = oneshot::channel();
        self.commands
            .try_send(Queued {
                workspace,
                attempt,
                command: make(reply),
            })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => ActorError::Busy,
                mpsc::error::TrySendError::Closed(_) => ActorError::Unavailable,
            })?;
        tokio::time::timeout(COMMAND_TIMEOUT, result)
            .await
            .map_err(|cause| ActorError::Timeout {
                effect: caller.cancel(),
                cause,
            })?
            .map_err(|cause| ActorError::Disconnected {
                effect: caller.cancel(),
                cause,
            })?
    }
}

impl Drop for Actor {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

async fn run(
    mut owner: Owner,
    mut requests: mpsc::Receiver<Queued>,
    stop: CancellationToken,
) -> Shutdown {
    if let Some((client, events)) = owner.controller() {
        let mut targets = Targets::default();
        let ended = loop {
            tokio::select! {
                () = stop.cancelled() => break BrowserFailure::Closed,
                next = requests.recv() => {
                    let Some(queued) = next else { break BrowserFailure::Closed };
                    if stop.is_cancelled() {
                        break BrowserFailure::Closed;
                    }
                    if queued.attempt.start() {
                        handle(&mut targets, client, queued).await;
                    }
                }
                next = events.next() => {
                    let Some(event) = next else { break BrowserFailure::NoEngine };
                    observe(&mut targets, &event).await;
                }
            }
        };
        targets.end(ended);
    }
    drop(requests);
    let receipt = owner.close().await;
    Shutdown {
        receipt,
        _owner: owner,
    }
}

async fn handle(targets: &mut Targets, client: &Client, queued: Queued) {
    let workspace = queued.workspace;
    match queued.command {
        Command::Open(url, size, reply) => {
            let result = targets
                .open(client, workspace, url, size)
                .await
                .map_err(ActorError::from);
            if let Err(Ok(browser)) = reply.send(result) {
                // The caller cannot attach a tab whose identity it never received.
                // An uncertain close leaves it owned until actor shutdown.
                let _closed = targets.close(workspace, &browser).await;
            }
        }
        Command::Watch(browser, reply) => {
            let result = async {
                let binding = targets.binding(workspace, &browser)?;
                let watch = binding.watch();
                binding.page.refresh_visible_frame().await?;
                Ok(watch)
            }
            .await;
            let _delivered = reply.send(result);
        }
        Command::Input(browser, revision, value, reply) => {
            let result = async {
                let page = targets.owned(workspace, &browser)?;
                let frame = page
                    .latest_frame()
                    .filter(|frame| frame.revision() == revision)
                    .ok_or(PageError::StaleFrame)?;
                page.input_from_frame(&frame, value).await?;
                Ok(())
            }
            .await;
            let _delivered = reply.send(result);
        }
        Command::Navigate(browser, url, reply) => {
            let result = async {
                let binding = targets.binding(workspace, &browser)?;
                let result = binding.page.navigate(url).await;
                let load = match &result {
                    Ok(()) => BrowserLoad::Loading,
                    Err(_) => BrowserLoad::Failed(BrowserFailure::Internal),
                };
                binding.announce(load);
                Ok(result?)
            }
            .await;
            let _delivered = reply.send(result);
        }
        Command::Close(browser, reply) => {
            let _delivered = reply.send(
                targets
                    .close(workspace, &browser)
                    .await
                    .map_err(ActorError::from),
            );
        }
    }
}

async fn observe(targets: &mut Targets, event: &Event) {
    let Some(binding) = event
        .session()
        .and_then(|session| targets.observing(session))
    else {
        return;
    };
    match event.kind() {
        EventKind::Navigated | EventKind::DocumentOpened | EventKind::WithinDocument => {
            if binding.page.invalidate().is_ok() {
                binding.announce(BrowserLoad::Loading);
            }
        }
        EventKind::Loaded | EventKind::Stopped => {
            binding.announce(BrowserLoad::Ready);
            let _captured = binding.page.refresh_visible_frame().await;
        }
        EventKind::Attached
        | EventKind::Detached
        | EventKind::Crashed
        | EventKind::Screencast
        | EventKind::Dialog
        | EventKind::Console
        | EventKind::Exception
        | EventKind::Request
        | EventKind::Response
        | EventKind::RequestFailed
        | EventKind::RequestPaused
        | EventKind::Download
        | EventKind::Unknown => {}
        EventKind::Loading => binding.announce(BrowserLoad::Loading),
    }
}
