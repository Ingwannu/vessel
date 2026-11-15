//! Cache ownership is installed before a load can be polled. Completion is
//! retained by the node even if every external receiver was dropped.
use super::core::{Completion, Decision, Read, ResultValue};
use super::{
    Active, Arc, CacheError, CancellationToken, Delivery, Future, Key, Mutex, Owner, PoisonError,
    ReadReason, Reader, Reply, Snapshot, State, WaitMode, WorkspaceError, core, watch,
};
use tokio::time::Instant;

impl Reader {
    pub(super) fn prepare<F, Load>(
        &self,
        key: Key,
        reason: ReadReason,
        owner: Owner<'_>,
        load: Load,
    ) -> Result<Delivery, CacheError>
    where
        F: Future<Output = Result<Snapshot, WorkspaceError>> + Send + 'static,
        Load: FnOnce(CancellationToken) -> F,
    {
        if owner.stop.is_cancelled() {
            return Ok(Delivery::Complete(Err(Arc::new(WorkspaceError::from(
                super::super::jobs::JobError::Closed,
            )))));
        }
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let mut cancelled = match reason {
            ReadReason::View => Vec::new(),
            ReadReason::Force(reason) => {
                tracing::debug!(?reason, "forced comparison refresh");
                state.cache.invalidate_siblings(&key)
            }
        };
        let Decision { read, cancel } = state.cache.read(key, reason, Instant::now())?;
        cancelled.extend(cancel);
        for cancel in cancelled {
            let active = state
                .active
                .iter()
                .find(|active| active.id == cancel)
                .ok_or(CacheError::Ownership)?;
            active.cancel.cancel();
        }
        match read {
            Read::Hit(value) => Ok(Delivery::Complete(Ok(value))),
            Read::Start(id) => Ok(self.start(&mut state, id, owner, load)),
            Read::Join(id) => state.waiting(id, WaitMode::Join),
            Read::After(id) => state.waiting(id, WaitMode::After),
        }
    }

    fn start<F, Load>(
        &self,
        state: &mut State,
        id: core::BuildId,
        owner: Owner<'_>,
        load: Load,
    ) -> Delivery
    where
        F: Future<Output = Result<Snapshot, WorkspaceError>> + Send + 'static,
        Load: FnOnce(CancellationToken) -> F,
    {
        let cancel = owner.stop.child_token();
        let (sender, reply) = watch::channel(None);
        state.active.push(Active {
            id,
            cancel: cancel.clone(),
            reply: reply.clone(),
        });
        let pending = match self
            .jobs
            .submit(owner.tasks, owner.stop.clone(), load(cancel))
        {
            Ok(pending) => pending,
            Err(error) => {
                let _finished = state.cache.finish(id, ResultValue::Failed, Instant::now());
                state.active.retain(|active| active.id != id);
                return Delivery::Complete(Err(Arc::new(WorkspaceError::from(error))));
            }
        };
        let retained = Arc::clone(&self.state);
        let _settler = owner.tasks.spawn(async move {
            let result = match pending.result().await {
                Ok(result) => result,
                Err(error) => Err(WorkspaceError::from(error)),
            };
            finish(&retained, id, &sender, result);
        });
        Delivery::Wait {
            reply,
            mode: WaitMode::Join,
        }
    }
}

fn finish(
    retained: &Mutex<State>,
    id: core::BuildId,
    sender: &watch::Sender<Option<Reply>>,
    result: Result<Snapshot, WorkspaceError>,
) {
    let result = result.map(Arc::new).map_err(Arc::new);
    let cached = match &result {
        Ok(value) => ResultValue::Value(Arc::clone(value)),
        Err(_) => ResultValue::Failed,
    };
    let mut state = retained.lock().unwrap_or_else(PoisonError::into_inner);
    match state.cache.finish(id, cached, Instant::now()) {
        Completion::Published | Completion::Retired | Completion::Failed => {}
        Completion::Unknown => tracing::warn!("comparison completed without its cache reservation"),
    }
    state.active.retain(|active| active.id != id);
    let _prior = sender.send_replace(Some(result));
}

impl State {
    fn waiting(&self, id: core::BuildId, mode: WaitMode) -> Result<Delivery, CacheError> {
        let active = self
            .active
            .iter()
            .find(|active| active.id == id)
            .ok_or(CacheError::Ownership)?;
        Ok(Delivery::Wait {
            reply: active.reply.clone(),
            mode,
        })
    }
}
