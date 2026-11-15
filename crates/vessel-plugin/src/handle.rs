//! Bounded caller admission and cancellation that survives a full command queue.

use std::sync::Arc;

use tokio::sync::{Notify, Semaphore, mpsc, oneshot, watch};
use tokio_util::sync::CancellationToken;
use vessel_proto::plugins::{HelperCall, HelperPayload, PluginAddress};

use crate::runtime_types::{
    CALL_OVERHEAD_BYTES, CallError, MAX_ADMITTED_BYTES, REQUEST_DEADLINE, Request, RuntimeState,
};

/// Cloneable request entry, not an installation/configuration capability. Node
/// authorization precedes this call; the actor rechecks captured generation.
#[derive(Debug, Clone)]
pub struct RuntimeHandle {
    pub(crate) requests: mpsc::Sender<Request>,
    pub(crate) state: watch::Receiver<RuntimeState>,
    pub(crate) cancelled: Arc<Notify>,
    pub(crate) bytes: Arc<Semaphore>,
    pub(crate) calls: Arc<Semaphore>,
}

impl RuntimeHandle {
    /// Subscribe to atomic catalog replacement/withdrawal. Retained snapshots
    /// are display data; callbacks always pass through the live actor again.
    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<RuntimeState> {
        self.state.clone()
    }

    /// Invoke without waiting for a queue slot. Count and weighted-byte permits
    /// remain owned by the actor until callback settlement, even after caller
    /// cancellation, so an uncooperative helper cannot accumulate work.
    ///
    /// # Errors
    /// Returns a safe class for capacity, cancellation, stale generation,
    /// timeout, callback refusal or process loss. Never exposes helper text.
    pub async fn call(
        &self,
        address: PluginAddress,
        call: HelperCall,
        cancel: &CancellationToken,
    ) -> Result<HelperPayload, CallError> {
        if cancel.is_cancelled() {
            return Err(CallError::Cancelled);
        }
        let count = Arc::clone(&self.calls)
            .try_acquire_owned()
            .map_err(|_busy| CallError::Busy)?;
        let size = call_bytes(&call)?;
        let bytes = Arc::clone(&self.bytes)
            .try_acquire_many_owned(size)
            .map_err(|_busy| CallError::Busy)?;
        let deadline = tokio::time::Instant::now()
            .checked_add(REQUEST_DEADLINE)
            .ok_or(CallError::Timeout)?;
        let local = CancellationToken::new();
        let _guard = CancelOnDrop {
            token: local.clone(),
            wake: Arc::clone(&self.cancelled),
        };
        let (reply, receive) = oneshot::channel();
        let request = Request {
            address,
            call,
            reply,
            deadline,
            cancel: local,
            bytes,
            count,
        };
        self.requests
            .try_send(request)
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_request) => CallError::Busy,
                mpsc::error::TrySendError::Closed(_request) => CallError::Closed,
            })?;
        tokio::select! {
            () = cancel.cancelled() => Err(CallError::Cancelled),
            _elapsed = tokio::time::sleep_until(deadline) => Err(CallError::Timeout),
            result = receive => result.map_err(|_gone| CallError::Closed)?,
        }
    }
}

fn call_bytes(call: &HelperCall) -> Result<u32, CallError> {
    let payload = match call {
        HelperCall::Invoke { input, .. } | HelperCall::Hook { input, .. } => input,
    };
    let count = payload
        .raw()
        .as_text()
        .len()
        .checked_add(CALL_OVERHEAD_BYTES)
        .ok_or(CallError::Busy)?;
    if count > MAX_ADMITTED_BYTES {
        return Err(CallError::Busy);
    }
    u32::try_from(count).map_err(|_range| CallError::Busy)
}

struct CancelOnDrop {
    token: CancellationToken,
    wake: Arc<Notify>,
}

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.token.cancel();
        self.wake.notify_one();
    }
}
