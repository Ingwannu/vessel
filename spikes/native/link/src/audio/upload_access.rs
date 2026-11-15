//! Capture-origin upload permission: a mute interval never reopens for an old frame.
//!
//! Snapshot before buffering microphone samples, not when uploading them. This
//! prevents buffered sound captured while muted from entering a later unmute.

use crate::cancel::Fence;
use std::sync::Arc;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::sync::{Mutex, TryLockError};
#[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
use vessel_app_view::voice::Mute;

#[cfg(any(target_os = "linux", target_os = "windows"))]
#[derive(Debug, Clone)]
pub(super) struct Gate {
    current: Arc<Mutex<Option<Arc<Fence>>>>,
    stopped: Arc<Fence>,
}
#[cfg(any(target_os = "linux", target_os = "windows"))]
impl Gate {
    pub(super) fn new(stopped: Arc<Fence>) -> Self {
        Self {
            current: Arc::new(Mutex::new(Some(Arc::new(Fence::default())))),
            stopped,
        }
    }
    /// Callback never waits for a control lock: a concurrent transition suppresses
    /// this block. Poisoning fences the acquisition instead of guessing access.
    pub(super) fn snapshot(&self) -> Access {
        let interval = if self.stopped.is_closed() {
            None
        } else {
            match self.current.try_lock() {
                Ok(current) => current.as_ref().map(|permission| Interval {
                    permission: Arc::clone(permission),
                    stopped: Arc::clone(&self.stopped),
                }),
                Err(TryLockError::WouldBlock) => None,
                Err(TryLockError::Poisoned(_private)) => {
                    self.stopped.close();
                    None
                }
            }
        };
        Access(interval)
    }
    #[cfg(test)]
    pub(super) fn set_mute(&self, mute: Mute) -> Result<(), MuteError> {
        if self.stopped.is_closed() {
            return Err(MuteError::Stopped);
        }
        let mut current = self.current.lock().map_err(|_private| {
            self.stopped.close();
            MuteError::Ownership
        })?;
        match mute {
            Mute::Muted => {
                if let Some(interval) = current.take() {
                    interval.close();
                }
            }
            Mute::Open => {
                if current.is_none() {
                    *current = Some(Arc::new(Fence::default()));
                }
            }
        }
        Ok(())
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum MuteError {
    #[error("capture is stopped")]
    Stopped,
    #[error("capture upload ownership cannot be verified")]
    Ownership,
}

#[derive(Debug)]
pub(super) struct Access(Option<Interval>);
impl Access {
    pub(super) fn live(&self) -> bool {
        self.0.as_ref().is_some_and(|interval| !interval.stopped())
    }
    #[cfg(test)]
    pub(super) fn interval(&self) -> Option<Interval> {
        self.0.clone()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Interval {
    permission: Arc<Fence>,
    stopped: Arc<Fence>,
}
impl Interval {
    pub(crate) fn stopped(&self) -> bool {
        self.permission.is_closed() || self.stopped.is_closed()
    }
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) async fn closed(&self) {
        tokio::select! { biased; () = self.stopped.closed() => {}, () = self.permission.closed() => {} }
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
mod tests;
