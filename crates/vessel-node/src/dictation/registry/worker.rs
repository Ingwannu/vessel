//! One id's serial generations; real preparation and actor teardown retain quota.
use super::{Caller, Entry, Phase, RegistryError, Result, Snapshot, Start, State, remove_current};
use crate::dictation::{
    commands::Handle,
    control::Control,
    prepare::{Factory, PrepareError},
    quota::Lease,
    stream_error::StreamError,
};
use crate::dictation::{
    control::Retention,
    stream::{self, CloseSignals},
};
use std::sync::Arc;
use std::sync::Weak;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use vessel_proto::dictation::{DictationId, Pcm16Mono};

pub(super) struct Launch {
    pub(super) id: DictationId,
    pub(super) entry: Arc<Entry>,
    pub(super) caller: Caller,
    pub(super) lease: Arc<Lease>,
    pub(super) starts: mpsc::Receiver<Start>,
    pub(super) command: Start,
    pub(super) control: Control<StreamError>,
}
struct Worker {
    state: Weak<State>,
    id: DictationId,
    entry: Arc<Entry>,
    caller: Caller,
    lease: Option<Arc<Lease>>,
    starts: mpsc::Receiver<Start>,
    factory: Arc<Factory>,
    snapshot: Snapshot,
    closed: CancellationToken,
}
struct Active {
    handle: Handle,
    task: Option<JoinHandle<()>>,
}
impl Drop for Active {
    fn drop(&mut self) {
        // Dropping a JoinHandle detaches. A supervisor unwind must instead
        // retire its actor; the actor itself also owns a reservation lease.
        let _first = self
            .handle
            .control
            .stop(Arc::new(StreamError::Cancelled), Retention::Forget);
    }
}
impl Active {
    async fn retire(mut self) -> Result<()> {
        if !self.handle.closed.is_cancelled() {
            let _first = self
                .handle
                .control
                .stop(Arc::new(StreamError::Cancelled), Retention::Forget);
            self.handle.closed.cancelled().await;
        }
        if let Some(task) = self.task.take() {
            task.await
                .map_err(|source| Arc::new(RegistryError::Worker(source)))?;
        }
        Ok(())
    }
}
pub(super) fn spawn(state: &Arc<State>, launch: Launch) {
    let completed = launch.entry.finished.clone().drop_guard();
    let worker = Worker {
        state: Arc::downgrade(state),
        id: launch.id,
        entry: launch.entry,
        caller: launch.caller,
        lease: Some(launch.lease),
        starts: launch.starts,
        factory: Arc::clone(&state.factory),
        snapshot: state.snapshot.clone(),
        closed: state.closed.clone(),
    };
    drop(state.tasks.spawn(async move {
        let _completed = completed;
        worker.run(launch.command, launch.control).await;
    }));
}
impl Worker {
    fn closing(&self) -> bool {
        self.entry.stop.is_cancelled()
            || self.caller.closed.is_cancelled()
            || self.closed.is_cancelled()
    }
    fn stop(control: &Control<StreamError>, incoming: StreamError) -> Arc<StreamError> {
        control.stop(Arc::new(incoming), Retention::Forget)
    }
    async fn run(mut self, mut command: Start, mut control: Control<StreamError>) {
        loop {
            drop(
                self.entry
                    .phase
                    .send_replace(Phase::Preparing(control.clone())),
            );
            let active = self
                .prepare(&mut command.reply, command.format, &control)
                .await;
            match active {
                Ok(active) => {
                    drop(
                        self.entry
                            .phase
                            .send_replace(Phase::Live(active.handle.clone())),
                    );
                    if command.reply.send(Ok(())).is_err() {
                        let _first = Self::stop(&control, StreamError::Cancelled);
                    } else {
                        self.wait_live(&active.handle).await;
                    }
                    drop(
                        self.entry
                            .phase
                            .send_replace(Phase::Retiring(control.clone())),
                    );
                    if let Err(error) = active.retire().await {
                        self.refuse_pending(&error);
                        self.entry.stop.cancel();
                    }
                }
                Err(error) => drop(command.reply.send(Err(error))),
            }
            let Some(next) = self.next_or_remove().await else {
                break;
            };
            command = next;
            control = Control::new(&self.caller.closed);
        }
    }
    async fn prepare(
        &mut self,
        reply: &mut oneshot::Sender<Result<()>>,
        format: Pcm16Mono,
        control: &Control<StreamError>,
    ) -> Result<Active> {
        if self.closing() || reply.is_closed() {
            return Err(Arc::new(RegistryError::Stream(Self::stop(
                control,
                StreamError::Cancelled,
            ))));
        }
        let lease = self
            .lease
            .clone()
            .ok_or_else(|| Arc::new(RegistryError::Gone))?;
        let factory = Arc::clone(&self.factory);
        let snapshot = self.snapshot.clone();
        let stopped = control.work();
        let preparing = async move {
            let current = snapshot.current().await;
            factory.prepare(&current, lease, &stopped).await
        };
        let source = self.await_preparation(reply, control, preparing).await?;
        let lease = self
            .lease
            .clone()
            .ok_or_else(|| Arc::new(RegistryError::Gone))?;
        let outbound = self.caller.outbound.clone();
        let (handle, task) = stream::spawn(
            format.rate(),
            source,
            CloseSignals {
                owner: self.caller.closed.clone(),
                node: self.closed.clone(),
            },
            async move { outbound.closed().await },
            control.clone(),
            lease,
        )
        .map_err(|source| Arc::new(RegistryError::Stream(source)))?;
        Ok(Active {
            handle,
            task: Some(task),
        })
    }
    async fn await_preparation<F>(
        &self,
        reply: &mut oneshot::Sender<Result<()>>,
        control: &Control<StreamError>,
        preparing: F,
    ) -> Result<crate::dictation::jobs::Source>
    where
        F: Future<Output = std::result::Result<crate::dictation::jobs::Source, PrepareError>>,
    {
        tokio::pin!(preparing);
        let source = loop {
            tokio::select! {
                biased;
                () = self.entry.stop.cancelled() => break None,
                () = self.caller.closed.cancelled() => break None,
                () = self.closed.cancelled() => break None,
                () = self.caller.outbound.closed() => { self.entry.stop.cancel(); break None; }
                () = reply.closed() => break None,
                () = self.entry.changed.notified() => {
                    if !self.starts.is_empty() { let _first = Self::stop(control, StreamError::Replaced); break None; }
                }
                result = &mut preparing => break Some(result),
            }
        };
        let Some(source) = source else {
            let first = control
                .chosen()
                .unwrap_or_else(|| Self::stop(control, StreamError::Cancelled));
            // A real blocking read/native-root load cannot be aborted. The id
            // and lease stay here until it ends, before any replacement reads.
            let _discarded = preparing.await;
            return Err(Arc::new(RegistryError::Stream(first)));
        };
        if let Some(first) = control.chosen() {
            return Err(Arc::new(RegistryError::Stream(first)));
        }
        if self.closing() || reply.is_closed() {
            return Err(Arc::new(RegistryError::Stream(Self::stop(
                control,
                StreamError::Cancelled,
            ))));
        }
        let source = source.map_err(|source| Arc::new(RegistryError::Prepare(source)))?;
        Ok(source)
    }
    async fn wait_live(&self, handle: &Handle) {
        loop {
            tokio::select! {
                biased;
                () = self.entry.stop.cancelled() => { let _first = Self::stop(&handle.control, StreamError::Cancelled); break; }
                () = self.caller.closed.cancelled() => { let _first = Self::stop(&handle.control, StreamError::Cancelled); break; }
                () = self.closed.cancelled() => { let _first = Self::stop(&handle.control, StreamError::Cancelled); break; }
                () = self.caller.outbound.closed() => { self.entry.stop.cancel(); let _first = Self::stop(&handle.control, StreamError::Cancelled); break; }
                () = self.entry.changed.notified() => {
                    if !self.starts.is_empty() { let _first = Self::stop(&handle.control, StreamError::Replaced); break; }
                }
                () = handle.closed.cancelled() => break,
            }
        }
    }
    fn refuse_pending(&mut self, error: &Arc<RegistryError>) {
        self.starts.close();
        while let Ok(command) = self.starts.try_recv() {
            drop(command.reply.send(Err(Arc::clone(error))));
        }
    }
    async fn next_or_remove(&mut self) -> Option<Start> {
        let Some(state) = self.state.upgrade() else {
            self.refuse_pending(&Arc::new(RegistryError::Gone));
            drop(self.lease.take());
            self.entry.finished.cancel();
            return None;
        };
        let mut entries = state.entries.lock().await;
        if !self.closing()
            && let Ok(command) = self.starts.try_recv()
        {
            return Some(command);
        }
        self.refuse_pending(&Arc::new(RegistryError::Stream(Arc::new(
            StreamError::Cancelled,
        ))));
        remove_current(&mut entries, &self.id, &self.entry);
        // Removal, final permit release and finish become visible before a
        // new start can enter the map; stale work cannot remove its successor.
        drop(self.lease.take());
        self.entry.finished.cancel();
        None
    }
}
