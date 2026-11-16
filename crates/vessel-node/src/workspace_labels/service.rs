//! Serialize durable mutations and publication independently of caller lifetime.
//!
//! [Decision Log]
//! - Purpose: cancellation after a durable write must not omit its catalog event.
//! - Constraint: a request future can disappear while the single writer commits.
//! - Alternatives: a request-held mutex (cancellation drops the publication step),
//!   or a second journal store (duplicates the existing database transaction).
//! - Choice: one bounded actor awaits each commit and publishes before replying;
//!   observation subscribes before asking that actor for its current cursor.
//! - Effect: disconnected callers release their poll, not accepted mutations.
//!   The real store adapter and scripted timing fixtures share this boundary.

use std::future::Future;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot, watch};
use vessel_proto::workspace_labels::{
    AssignWorkspaceLabel, CatalogCursor, CatalogList, CatalogRead, DeleteWorkspaceLabel,
    InspectWorkspaceLabelDelete, Label, ListWorkspaceLabels, UpdateWorkspaceLabel,
    WorkspaceLabelAssigned, WorkspaceLabelDeleted, WorkspaceLabelUpdated,
};
use vessel_store::StoreError;

use super::journal::Journal;

const COMMANDS: usize = 32;
const OBSERVE_LIMIT: Duration = Duration::from_secs(30);

pub(super) type Committed<T> = vessel_store::WorkspaceLabelCommit<T>;

/// The real adapter only returns after the existing store writer commits.
pub(super) trait Backend: Send + 'static {
    fn catalog(&self) -> impl Future<Output = Result<Vec<Label>, StoreError>> + Send;
    fn assign(
        &self,
        request: AssignWorkspaceLabel,
    ) -> impl Future<Output = Result<Committed<WorkspaceLabelAssigned>, StoreError>> + Send;
    fn update(
        &self,
        request: UpdateWorkspaceLabel,
    ) -> impl Future<Output = Result<Committed<WorkspaceLabelUpdated>, StoreError>> + Send;
    fn inspect(
        &self,
        request: InspectWorkspaceLabelDelete,
    ) -> impl Future<Output = Result<WorkspaceLabelDeleted, StoreError>> + Send;
    fn delete(
        &self,
        request: DeleteWorkspaceLabel,
    ) -> impl Future<Output = Result<Committed<WorkspaceLabelDeleted>, StoreError>> + Send;
}

enum Command {
    Close(oneshot::Sender<()>),
    List(Option<CatalogCursor>, oneshot::Sender<CatalogList>),
    Assign(
        AssignWorkspaceLabel,
        oneshot::Sender<Result<WorkspaceLabelAssigned, StoreError>>,
    ),
    Update(
        UpdateWorkspaceLabel,
        oneshot::Sender<Result<WorkspaceLabelUpdated, StoreError>>,
    ),
    Inspect(
        InspectWorkspaceLabelDelete,
        oneshot::Sender<Result<WorkspaceLabelDeleted, StoreError>>,
    ),
    Delete(
        DeleteWorkspaceLabel,
        oneshot::Sender<Result<WorkspaceLabelDeleted, StoreError>>,
    ),
}

#[derive(Debug, Clone)]
pub(crate) struct Labels {
    commands: mpsc::Sender<Command>,
    changed: watch::Receiver<CatalogCursor>,
}

impl Labels {
    pub(super) async fn start(backend: impl Backend) -> Result<Self, StoreError> {
        let journal = Journal::new(backend.catalog().await?);
        let (changed, receiver) = watch::channel(journal.cursor());
        let (commands, incoming) = mpsc::channel(COMMANDS);
        let _worker = tokio::spawn(run(backend, journal, changed, incoming));
        Ok(Self {
            commands,
            changed: receiver,
        })
    }

    pub(crate) async fn list(
        &self,
        request: ListWorkspaceLabels,
    ) -> Result<CatalogList, StoreError> {
        // Register first: a commit between snapshot and wait remains observable.
        let mut changes = self.changed.clone();
        let _seen = *changes.borrow_and_update();
        let snapshot = self.snapshot(request.cursor).await?;
        if request.read == CatalogRead::Now || request.cursor != Some(snapshot.head) {
            return Ok(snapshot);
        }
        match tokio::time::timeout(OBSERVE_LIMIT, changes.changed()).await {
            Ok(Ok(())) => self.snapshot(request.cursor).await,
            Ok(Err(_closed)) => Err(StoreError::Closed),
            Err(_elapsed) => Ok(snapshot),
        }
    }

    /// Drain changes accepted before this marker before the node closes its
    /// store. Later commands refuse Closed, as the store writer's close does.
    pub(crate) async fn close(&self) -> Result<(), StoreError> {
        let (answer, reply) = oneshot::channel();
        self.commands
            .send(Command::Close(answer))
            .await
            .map_err(|_closed| StoreError::Closed)?;
        reply.await.map_err(|_closed| StoreError::Closed)
    }

    // Queue/reply channel errors mean closure only. Do not retain a rejected
    // command payload as diagnostic text; backend errors keep their own source.
    async fn snapshot(&self, cursor: Option<CatalogCursor>) -> Result<CatalogList, StoreError> {
        let (answer, reply) = oneshot::channel();
        self.commands
            .send(Command::List(cursor, answer))
            .await
            .map_err(|_closed| StoreError::Closed)?;
        reply.await.map_err(|_closed| StoreError::Closed)
    }

    pub(crate) async fn assign(
        &self,
        request: AssignWorkspaceLabel,
    ) -> Result<WorkspaceLabelAssigned, StoreError> {
        let (answer, reply) = oneshot::channel();
        self.commands
            .send(Command::Assign(request, answer))
            .await
            .map_err(|_closed| StoreError::Closed)?;
        reply.await.map_err(|_closed| StoreError::Closed)?
    }

    pub(crate) async fn update(
        &self,
        request: UpdateWorkspaceLabel,
    ) -> Result<WorkspaceLabelUpdated, StoreError> {
        let (answer, reply) = oneshot::channel();
        self.commands
            .send(Command::Update(request, answer))
            .await
            .map_err(|_closed| StoreError::Closed)?;
        reply.await.map_err(|_closed| StoreError::Closed)?
    }

    pub(crate) async fn inspect(
        &self,
        request: InspectWorkspaceLabelDelete,
    ) -> Result<WorkspaceLabelDeleted, StoreError> {
        let (answer, reply) = oneshot::channel();
        self.commands
            .send(Command::Inspect(request, answer))
            .await
            .map_err(|_closed| StoreError::Closed)?;
        reply.await.map_err(|_closed| StoreError::Closed)?
    }

    pub(crate) async fn delete(
        &self,
        request: DeleteWorkspaceLabel,
    ) -> Result<WorkspaceLabelDeleted, StoreError> {
        let (answer, reply) = oneshot::channel();
        self.commands
            .send(Command::Delete(request, answer))
            .await
            .map_err(|_closed| StoreError::Closed)?;
        reply.await.map_err(|_closed| StoreError::Closed)?
    }
}

async fn run(
    backend: impl Backend,
    mut journal: Journal,
    changed: watch::Sender<CatalogCursor>,
    mut incoming: mpsc::Receiver<Command>,
) {
    while let Some(command) = incoming.recv().await {
        match command {
            Command::Close(answer) => {
                incoming.close();
                let _cancelled = answer.send(());
                break;
            }
            Command::List(cursor, answer) => {
                let _cancelled = answer.send(journal.synchronize(cursor));
            }
            Command::Assign(request, answer) => {
                let result = publish(backend.assign(request).await, &mut journal, &changed);
                let _cancelled = answer.send(result);
            }
            Command::Update(request, answer) => {
                let result = publish(backend.update(request).await, &mut journal, &changed);
                let _cancelled = answer.send(result);
            }
            Command::Inspect(request, answer) => {
                let _cancelled = answer.send(backend.inspect(request).await);
            }
            Command::Delete(request, answer) => {
                let result = publish(backend.delete(request).await, &mut journal, &changed);
                let _cancelled = answer.send(result);
            }
        }
    }
}

fn publish<T>(
    result: Result<Committed<T>, StoreError>,
    journal: &mut Journal,
    changed: &watch::Sender<CatalogCursor>,
) -> Result<T, StoreError> {
    let committed = result?;
    if let Some(delta) = committed.delta {
        let update = journal.committed(delta);
        let _previous = changed.send_replace(update.cursor);
    }
    Ok(committed.result)
}

#[cfg(test)]
mod tests;
