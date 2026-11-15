//! Staged profile-wide input ordering, below real node/target admission.
//!
//! `browser: wire the qualified node browser actor` removes the outer cfg(test)
//! only with the actual driver and cleanup proof. A driver must revalidate the
//! owned target/context before effects; metadata/caller ids cannot authorize it.
//! Closing/drain must retain real process/profile resources on unconfirmed cleanup.
mod attempt;
#[cfg(test)]
mod tests;

use attempt::{Attempt, Caller};
use std::future::Future;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{Mutex, Semaphore, oneshot, watch};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_proto::browser::{BrowserEffect, COMMAND_TIMEOUT};

/// Includes active and queued typed operations; permits live in actual workers.
const MAX_INPUT_TRANSACTIONS: usize = 64;

pub(crate) trait Driver: Send + Sync + 'static {
    type Target: Send + Sync + 'static;
    type Command: Send + 'static;
    type Error: std::error::Error + Send + Sync + 'static;
    fn activate(
        &self,
        target: &Self::Target,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn apply(
        &self,
        target: &Self::Target,
        command: Self::Command,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
    /// This is a trusted owner contract, not a metadata receipt. It must close
    /// commands/pipes/streams/children and await the actual cleanup boundary.
    fn stop_and_drain(&self) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
#[derive(thiserror::Error)]
pub(crate) enum InputError<E: std::error::Error + Send + Sync + 'static> {
    #[error("browser input capacity is occupied")]
    Busy,
    #[error("browser profile input is closed")]
    Closed,
    #[error("browser input timed out ({effect:?})")]
    TimedOut {
        effect: BrowserEffect,
        #[source]
        cause: tokio::time::error::Elapsed,
    },
    #[error("browser input result was lost ({effect:?})")]
    Disconnected { effect: BrowserEffect },
    #[error("browser input failed with uncertain effects")]
    Uncertain {
        #[source]
        cause: Arc<E>,
    },
}
impl<E: std::error::Error + Send + Sync + 'static> std::fmt::Debug for InputError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BrowserInputError({self})")
    }
}
enum Stop<E> {
    Pending,
    Complete(Result<(), Arc<E>>),
}
impl<E> Clone for Stop<E> {
    fn clone(&self) -> Self {
        match self {
            Self::Pending => Self::Pending,
            Self::Complete(result) => Self::Complete(result.clone()),
        }
    }
}
struct Shared<D: Driver> {
    driver: D,
    capacity: Arc<Semaphore>,
    serial: Mutex<()>,
    admission: Mutex<()>,
    closing: CancellationToken,
    tasks: TaskTracker,
    stop_started: AtomicBool,
    stop: watch::Sender<Stop<D::Error>>,
}
/// This lane is owned by one immutable browser profile and can never reopen.
pub(crate) struct InputLane<D: Driver>(Arc<Shared<D>>);
impl<D: Driver> std::fmt::Debug for InputLane<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BrowserInputLane([owned profile])")
    }
}
impl<D: Driver> Clone for InputLane<D> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
impl<D: Driver> InputLane<D> {
    pub(crate) fn new(driver: D) -> Self {
        Self::with_capacity(driver, MAX_INPUT_TRANSACTIONS)
    }
    fn with_capacity(driver: D, capacity: usize) -> Self {
        let (stop, _receiver) = watch::channel(Stop::Pending);
        Self(Arc::new(Shared {
            driver,
            capacity: Arc::new(Semaphore::new(capacity)),
            serial: Mutex::new(()),
            admission: Mutex::new(()),
            closing: CancellationToken::new(),
            tasks: TaskTracker::new(),
            stop_started: AtomicBool::new(false),
            stop,
        }))
    }
    pub(crate) async fn run(
        &self,
        target: D::Target,
        command: D::Command,
    ) -> Result<(), InputError<D::Error>> {
        self.run_with_budget(target, command, COMMAND_TIMEOUT).await
    }
    async fn run_with_budget(
        &self,
        target: D::Target,
        command: D::Command,
        budget: std::time::Duration,
    ) -> Result<(), InputError<D::Error>> {
        if self.0.closing.is_cancelled() {
            return Err(InputError::Closed);
        }
        let permit = Arc::clone(&self.0.capacity)
            .try_acquire_owned()
            .map_err(|_full| InputError::Busy)?;
        let (answer, result) = oneshot::channel();
        let delivered = Arc::new(Attempt::new());
        let caller = Caller::new(Arc::clone(&delivered));
        {
            // Serialize registration with shutdown; the tracker token exists
            // before this gate opens again, so an empty close cannot miss a task.
            let _registration = self.0.admission.lock().await;
            if self.0.closing.is_cancelled() {
                return Err(InputError::Closed);
            }
            let shared = Arc::clone(&self.0);
            let sent = Arc::clone(&delivered);
            let _task = self.0.tasks.spawn(async move {
                let _permit = permit;
                worker(shared, target, command, answer, sent).await;
            });
        }
        tokio::time::timeout(budget, result)
            .await
            .map_err(|cause| InputError::TimedOut {
                effect: caller.cancel(),
                cause,
            })?
            .map_err(|_gone| InputError::Disconnected {
                effect: caller.cancel(),
            })?
    }
    /// Caller cancellation drops only this waiter. Real stop work is separately
    /// registered and keeps its driver/resources until completion or quarantine.
    pub(crate) async fn shutdown(&self) -> Result<(), Arc<D::Error>> {
        {
            let _registration = self.0.admission.lock().await;
            self.0.closing.cancel();
            schedule_stop(&self.0);
            let _closed = self.0.tasks.close();
        }
        self.0.tasks.wait().await;
        stopped(&self.0).await
    }
}
fn schedule_stop<D: Driver>(shared: &Arc<Shared<D>>) {
    shared.closing.cancel();
    if shared.stop_started.swap(true, Ordering::AcqRel) {
        return;
    }
    let owner = Arc::clone(shared);
    // Called under the admission gate or by an already tracked worker. Its
    // existing token cannot disappear before this actual stop task is tracked.
    let _task = shared.tasks.spawn(async move {
        let result = owner.driver.stop_and_drain().await.map_err(Arc::new);
        let _previous = owner.stop.send_replace(Stop::Complete(result));
    });
}
async fn stopped<D: Driver>(shared: &Shared<D>) -> Result<(), Arc<D::Error>> {
    let mut state = shared.stop.subscribe();
    loop {
        let value = state.borrow_and_update().clone();
        if let Stop::Complete(result) = value {
            return result;
        }
        // The retained Shared owns the sender. Closure cannot be interpreted
        // as proof of cleanup, even if a future driver changes that ownership.
        if state.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}
async fn worker<D: Driver>(
    shared: Arc<Shared<D>>,
    target: D::Target,
    command: D::Command,
    mut answer: oneshot::Sender<Result<(), InputError<D::Error>>>,
    delivered: Arc<Attempt>,
) {
    // The Tokio guard covers activation and the whole input transaction: a
    // different target must not become active between a key down and key up.
    let _serial = tokio::select! {
        biased;
        ()=shared.closing.cancelled()=>return,
        ()=answer.closed()=>return,
        guard=shared.serial.lock()=>guard,
    };
    if shared.closing.is_cancelled() || answer.is_closed() || !delivered.start() {
        return;
    }
    let operation = async {
        shared.driver.activate(&target).await?;
        shared.driver.apply(&target, command).await
    };
    let result = tokio::select! {
        biased;
        ()=shared.closing.cancelled()=>None,
        ()=answer.closed()=>None,
        result=operation=>Some(result),
    };
    match result {
        Some(Ok(())) => {
            if answer.send(Ok(())).is_ok() {
                return;
            }
            schedule_stop(&shared);
            let _retained_stop = stopped(&shared).await;
        }
        Some(Err(cause)) => {
            schedule_stop(&shared);
            let _retained_stop = stopped(&shared).await;
            let _caller_gone = answer.send(Err(InputError::Uncertain {
                cause: Arc::new(cause),
            }));
        }
        None => {
            schedule_stop(&shared);
            let _retained_stop = stopped(&shared).await;
        }
    }
}
