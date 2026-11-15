//! One admission budget covers queued commands and retained raw PCM in the Pump.
//!
//! Speech intervals close synchronously before a barge-in wakes the worker.
//! A late wake only prunes fenced work; it never interrupts a newer interval.

use std::sync::Arc;
use std::sync::Mutex;

use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore, mpsc, oneshot};

use super::{
    packet::Packet,
    pump::Pump,
    receipts::{Handle, MAX_SEGMENTS, Refusal},
};
use crate::cancel::Fence;

/// Callbacks inspect these atomics without borrowing the command/watch state.
#[derive(Debug, Clone)]
pub(super) struct Access {
    interval: Arc<Fence>,
    request: Arc<Fence>,
}
impl Access {
    pub(super) fn stopped(&self) -> bool {
        self.interval.is_closed() || self.request.is_closed()
    }
}

#[derive(Debug, Clone)]
pub(in crate::audio) struct Control {
    stopped: Arc<Fence>,
    interval: Arc<Mutex<Arc<Fence>>>,
    admissions: Arc<Semaphore>,
    sender: mpsc::Sender<Command>,
    wake: Arc<Notify>,
}
#[derive(Debug)]
pub(in crate::audio) struct Inbox {
    receiver: mpsc::Receiver<Command>,
    wake: Arc<Notify>,
}
#[derive(Debug)]
pub(in crate::audio) struct Command {
    packet: Packet,
    response: oneshot::Sender<Result<Handle, Refusal>>,
}
#[derive(Debug)]
pub(in crate::audio) struct Pending {
    response: oneshot::Receiver<Result<Handle, Refusal>>,
    cancel: Option<Arc<Fence>>,
    stopped: Arc<Fence>,
    wake: Arc<Notify>,
}

impl Control {
    pub(in crate::audio) fn channel(stopped: Arc<Fence>) -> (Self, Inbox) {
        let (sender, receiver) = mpsc::channel(MAX_SEGMENTS);
        let wake = Arc::new(Notify::new());
        (
            Self {
                stopped,
                interval: Arc::new(Mutex::new(Arc::new(Fence::default()))),
                admissions: Arc::new(Semaphore::new(MAX_SEGMENTS)),
                sender,
                wake: Arc::clone(&wake),
            },
            Inbox { receiver, wake },
        )
    }
    /// Admission is synchronous/nonblocking; it covers PCM even after dequeue.
    pub(in crate::audio) fn submit(&self, packet: Packet) -> Result<Pending, Refusal> {
        if self.stopped.is_closed() {
            return Err(Refusal::Stopped);
        }
        let permit = Arc::clone(&self.admissions)
            .try_acquire_owned()
            .map_err(|_private| Refusal::Capacity)?;
        let interval = {
            let current = self.interval.lock().map_err(|_private| {
                self.stopped.close();
                Refusal::Stopped
            })?;
            Arc::clone(&*current)
        };
        let cancel = Arc::new(Fence::default());
        let access = Access {
            interval,
            request: Arc::clone(&cancel),
        };
        let (response, receiver) = oneshot::channel();
        let command = Command {
            packet: packet.admit(access, permit),
            response,
        };
        self.sender.try_send(command).map_err(|error| match error {
            mpsc::error::TrySendError::Full(_command) => Refusal::Capacity,
            mpsc::error::TrySendError::Closed(_command) => Refusal::Stopped,
        })?;
        Ok(Pending {
            response: receiver,
            cancel: Some(cancel),
            stopped: Arc::clone(&self.stopped),
            wake: Arc::clone(&self.wake),
        })
    }
    /// This fences prepared output as well as commands the worker has not read.
    pub(in crate::audio) fn interrupt(&self) -> Result<(), Refusal> {
        if self.stopped.is_closed() {
            return Err(Refusal::Stopped);
        }
        let mut interval = self.interval.lock().map_err(|_private| {
            self.stopped.close();
            Refusal::Stopped
        })?;
        interval.close();
        *interval = Arc::new(Fence::default());
        self.wake.notify_one();
        Ok(())
    }
    /// Local Stop never waits for a command slot or the physical worker.
    pub(in crate::audio) fn stop(&self) {
        self.stopped.close();
        self.wake.notify_one();
    }
}
impl Inbox {
    /// A wake is an instruction to prune fenced entries, never interrupt-all.
    pub(in crate::audio) async fn next(&mut self) -> Option<Command> {
        tokio::select! { biased;
            () = self.wake.notified() => None,
            command = self.receiver.recv() => command,
        }
    }
}
impl Command {
    pub(in crate::audio) fn apply(self, pump: &mut Pump) {
        // A cancelled receiver drops a produced Handle here, fencing its receipt.
        let _undelivered = self.response.send(pump.submit(self.packet));
    }
}
impl Pending {
    pub(in crate::audio) async fn ready(mut self) -> Result<Handle, Refusal> {
        let outcome = tokio::select! { biased;
            () = self.stopped.closed() => Err(Refusal::Stopped),
            response = &mut self.response => response.map_err(|_private| Refusal::Stopped)?,
        };
        if outcome.is_ok() {
            // The receipt now observes the same request fence; Pending no longer
            // owns cancellation. Dropping the returned Handle still interrupts it.
            let _transferred = self.cancel.take();
        }
        outcome
    }
}
impl Drop for Pending {
    fn drop(&mut self) {
        if let Some(cancel) = &self.cancel {
            cancel.close();
            self.wake.notify_one();
        }
    }
}

/// The permit is retained by Packet/Reader aliases, not by a completed receipt.
pub(super) type Reservation = Arc<OwnedSemaphorePermit>;

#[cfg(test)]
mod tests;
