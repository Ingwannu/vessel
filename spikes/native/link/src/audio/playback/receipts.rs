//! A segment finishes only after a validated drain event, or an explicit failure.

use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::watch;
use vessel_app_view::audio::{CaptureLease, CapturePurpose};

use super::packet::{Packet, Reader};
use crate::cancel::Fence;

pub(super) const MAX_SEGMENTS: usize = 2;
const MAX_DEVICE_DRAIN: Duration = Duration::from_secs(3);
const PREPARE_TIMEOUT: Duration = Duration::from_secs(10);
const PLAYBACK_GRACE: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Ticket(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::audio) struct Lease {
    capture: CaptureLease,
    ticket: Ticket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::audio) enum State {
    Queued,
    Rendering,
    Draining,
    Played,
    Interrupted,
    Failed,
}
impl State {
    fn terminal(self) -> bool {
        matches!(self, Self::Played | Self::Interrupted | Self::Failed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(in crate::audio) enum Refusal {
    #[error("playback is stopped")]
    Stopped,
    #[error("dictation cannot own conversation playback")]
    Purpose,
    #[error("playback queue is full")]
    Capacity,
    #[error("playback identities exhausted")]
    Exhausted,
    #[error("playback PCM format is invalid")]
    Format,
    #[error("playback completion arrived out of order")]
    Order,
    #[error("playback device clock is invalid")]
    Clock,
}

#[derive(Debug)]
pub(super) struct Receipt {
    stop: Fence,
    conversation: Arc<Fence>,
    state: watch::Sender<State>,
}
impl Receipt {
    pub(super) fn stopped(&self) -> bool {
        self.stop.is_closed() || self.conversation.is_closed() || self.state.borrow().terminal()
    }
    fn settle(&self, requested: State) {
        let _changed = self.state.send_if_modified(|state| {
            if state.terminal() {
                return false;
            }
            *state = if self.stopped() {
                State::Interrupted
            } else {
                requested
            };
            true
        });
    }
    fn interrupt(&self) {
        self.stop.close();
        self.settle(State::Interrupted);
    }
}

#[derive(Debug)]
pub(in crate::audio) struct Handle {
    lease: Lease,
    receipt: Arc<Receipt>,
}
impl Handle {
    pub(in crate::audio) fn lease(&self) -> Lease {
        self.lease
    }
    pub(in crate::audio) fn status(&self) -> watch::Receiver<State> {
        self.receipt.state.subscribe()
    }
    pub(in crate::audio) fn interrupt(&self) {
        self.receipt.interrupt();
    }
}
impl Drop for Handle {
    fn drop(&mut self) {
        self.interrupt();
    }
}

#[derive(Debug)]
struct Entry {
    packet: Packet,
    receipt: Arc<Receipt>,
    deadline: Instant,
}

#[derive(Debug)]
pub(in crate::audio) struct Book {
    capture: CaptureLease,
    conversation: Arc<Fence>,
    next: u64,
    entries: BTreeMap<Ticket, Entry>,
}
impl Book {
    pub(in crate::audio) fn new(
        capture: CaptureLease,
        conversation: Arc<Fence>,
    ) -> Result<Self, Refusal> {
        if capture.purpose() != CapturePurpose::Conversation {
            return Err(Refusal::Purpose);
        }
        Ok(Self {
            capture,
            conversation,
            next: 0,
            entries: BTreeMap::new(),
        })
    }
    pub(in crate::audio) fn submit(&mut self, packet: Packet) -> Result<Handle, Refusal> {
        if self.conversation.is_closed() {
            return Err(Refusal::Stopped);
        }
        if self.entries.len() >= MAX_SEGMENTS {
            return Err(Refusal::Capacity);
        }
        let deadline = Instant::now()
            .checked_add(PREPARE_TIMEOUT)
            .ok_or(Refusal::Clock)?;
        let next = self.next.checked_add(1).ok_or(Refusal::Exhausted)?;
        let ticket = Ticket(next);
        let (state, _observer) = watch::channel(State::Queued);
        let receipt = Arc::new(Receipt {
            stop: Fence::default(),
            conversation: Arc::clone(&self.conversation),
            state,
        });
        let _previous = self.entries.insert(
            ticket,
            Entry {
                packet,
                receipt: Arc::clone(&receipt),
                deadline,
            },
        );
        self.next = next;
        Ok(Handle {
            lease: Lease {
                capture: self.capture,
                ticket,
            },
            receipt,
        })
    }
    pub(super) fn queued(&self, lease: Lease) -> bool {
        lease.capture == self.capture
            && self.entries.get(&lease.ticket).is_some_and(|entry| {
                !entry.receipt.stopped() && *entry.receipt.state.borrow() == State::Queued
            })
    }
    pub(in crate::audio) fn begin(&mut self, lease: Lease) -> Result<Option<Reader>, Refusal> {
        let Some(entry) = self.entry(lease) else {
            return Ok(None);
        };
        if entry.receipt.stopped() {
            return Err(Refusal::Stopped);
        }
        if *entry.receipt.state.borrow() != State::Queued {
            return Err(Refusal::Order);
        }
        let now = Instant::now();
        if now >= entry.deadline {
            entry.receipt.settle(State::Failed);
            return Err(Refusal::Clock);
        }
        entry.deadline = entry
            .packet
            .duration()
            .checked_add(PLAYBACK_GRACE)
            .and_then(|duration| now.checked_add(duration))
            .ok_or(Refusal::Clock)?;
        entry.receipt.settle(State::Rendering);
        Ok(Some(entry.packet.reader(lease, Arc::clone(&entry.receipt))))
    }
    /// Called after the LAST output sample was copied by the device callback.
    /// `after` includes driver latency and the final sample's offset in that callback.
    pub(in crate::audio) fn scheduled(
        &mut self,
        lease: Lease,
        now: Instant,
        after: Option<Duration>,
    ) -> Result<(), Refusal> {
        let Some(entry) = self.entry(lease) else {
            return Ok(());
        };
        if entry.receipt.stopped() {
            return Err(Refusal::Stopped);
        }
        if *entry.receipt.state.borrow() != State::Rendering {
            return Err(Refusal::Order);
        }
        let Some(deadline) = after
            .filter(|delay| *delay <= MAX_DEVICE_DRAIN)
            .and_then(|delay| now.checked_add(delay))
        else {
            entry.receipt.settle(State::Failed);
            return Err(Refusal::Clock);
        };
        entry.deadline = deadline;
        entry.receipt.settle(State::Draining);
        Ok(())
    }
    /// The device actor sleeps until this deadline, alongside its callback/control channels.
    pub(in crate::audio) fn next_deadline(&self) -> Option<Instant> {
        self.entries.values().map(|entry| entry.deadline).min()
    }
    pub(in crate::audio) fn advance(&mut self, now: Instant) {
        self.entries.retain(|_ticket, entry| {
            if entry.receipt.stopped() {
                entry.receipt.settle(State::Interrupted);
            } else if now >= entry.deadline {
                let result = if *entry.receipt.state.borrow() == State::Draining {
                    State::Played
                } else {
                    State::Failed
                };
                entry.receipt.settle(result);
            }
            !entry.receipt.state.borrow().terminal()
        });
    }
    pub(in crate::audio) fn interrupt_all(&mut self) {
        for entry in self.entries.values() {
            entry.receipt.interrupt();
        }
        self.entries.clear();
    }
    fn entry(&mut self, lease: Lease) -> Option<&mut Entry> {
        if lease.capture != self.capture {
            return None;
        }
        self.entries.get_mut(&lease.ticket)
    }
}
impl Drop for Book {
    fn drop(&mut self) {
        for entry in self.entries.values() {
            entry.receipt.settle(State::Failed);
        }
    }
}

#[cfg(test)]
mod tests;
