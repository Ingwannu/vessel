//! Fakes-only barrier inside the actual executable filesystem lookup.
//! It observes/holds work, not quota, launch authority or command results.
use super::Role;
use crate::Lookup;
use crate::harnesses::{Structured, descriptor};
use std::num::NonZeroUsize;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::{mpsc, oneshot};
use vessel_proto::HarnessKind;

#[derive(Debug)]
pub(crate) struct Gate {
    binary: &'static str,
    entered: AtomicUsize,
    arrivals: mpsc::Sender<LookupArrival>,
}
impl Gate {
    pub(crate) fn enter(&self, binary: &str) {
        if binary != self.binary {
            return;
        }
        // Observation cannot wrap into a falsely small work count.
        let _old = self
            .entered
            .fetch_update(Ordering::AcqRel, Ordering::Relaxed, |value| {
                Some(value.saturating_add(1))
            });
        let (release, waiting) = oneshot::channel();
        if self
            .arrivals
            .blocking_send(LookupArrival {
                release: Some(release),
            })
            .is_ok()
        {
            let _released = waiting.blocking_recv();
        }
    }
}
/// Dropping an arrival releases its actual blocked lookup, including on panic.
#[derive(Debug)]
pub struct LookupArrival {
    release: Option<oneshot::Sender<()>>,
}
impl Drop for LookupArrival {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            let _gone = release.send(());
        }
    }
}
/// Physical lookup entries observed by this test barrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LookupCount(usize);
impl LookupCount {
    /// Inspect the count at a test assertion boundary.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}
/// Observes one selected fake program, with a bounded arrival channel.
#[derive(Debug)]
pub struct LookupBarrier {
    gate: Arc<Gate>,
    arrivals: mpsc::Receiver<LookupArrival>,
}
impl LookupBarrier {
    /// Wait for an actual blocking lookup, not a synthetic runtime callback.
    pub async fn arrived(&mut self) -> Option<LookupArrival> {
        self.arrivals.recv().await
    }
    /// How many selected physical lookups entered, including held ones.
    #[must_use]
    pub fn entered(&self) -> LookupCount {
        LookupCount(self.gate.entered.load(Ordering::Acquire))
    }
}
/// Hold the real lookup of one installed harness role. Exists only with fakes.
/// Closing the receiver releases queued arrivals; none can outlive a test.
#[must_use]
pub fn hold_lookup(
    lookup: Lookup,
    kind: HarnessKind,
    role: Role,
    capacity: NonZeroUsize,
) -> (Lookup, LookupBarrier) {
    let descriptor = descriptor(kind);
    let binary = match role {
        Role::Terminal => descriptor.binary,
        Role::Structured => match &descriptor.structured {
            Structured::Native { .. } => descriptor.binary,
            Structured::Adapter { binary, .. } => binary,
        },
    };
    let (send, arrivals) = mpsc::channel(capacity.get());
    let gate = Arc::new(Gate {
        binary,
        entered: AtomicUsize::new(0),
        arrivals: send,
    });
    (
        lookup.with_lookup_gate(Arc::clone(&gate)),
        LookupBarrier { gate, arrivals },
    )
}
