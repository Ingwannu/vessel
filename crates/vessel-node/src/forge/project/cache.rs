//! Finite owned page/flight state and refresh publication ordering.
//! A load allocation stays current after completion while older flights
//! exist, so their late completion cannot replace or resurrect its result.
use super::{Error, FRESH, MAX_KEYS, Mode, Outcome, Request};
use std::{collections::VecDeque, sync::Arc};
use tokio::time::Instant;
use vessel_forge::CredentialIdentity;
use vessel_proto::forge::{OpenPullPage, PullCursor, RepoRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Identity {
    Custom,
    Credential(CredentialIdentity),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Key {
    pub(super) repo: RepoRef,
    pub(super) after: Option<PullCursor>,
    pub(super) credential: Identity,
}
#[derive(Debug, Clone)]
pub(super) struct LoadId(Arc<()>);
impl LoadId {
    pub(super) fn new() -> Self {
        Self(Arc::new(()))
    }
    fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
pub(super) struct Cached {
    pub(super) page: Arc<OpenPullPage>,
    finished: Instant,
}
pub(super) struct Flight {
    id: LoadId,
    mode: Mode,
    pub(super) waiters: Vec<Request>,
}
pub(super) struct Slot {
    key: Key,
    current: Option<LoadId>,
    pub(super) cached: Option<Cached>,
    flights: Vec<Flight>,
}
impl Slot {
    pub(super) fn reuse(&self, mode: Mode) -> Option<Arc<OpenPullPage>> {
        self.cached
            .as_ref()
            .filter(|entry| mode == Mode::Cached && entry.finished.elapsed() < FRESH)
            .map(|entry| Arc::clone(&entry.page))
    }
    pub(super) fn joinable(&mut self, mode: Mode) -> Option<&mut Flight> {
        let current = self.current.as_ref()?;
        self.flights.iter_mut().find(|flight| {
            flight.id.same(current) && (mode == Mode::Cached || flight.mode == Mode::Refresh)
        })
    }
    pub(super) fn begin(&mut self, id: LoadId, request: Request) {
        self.current = Some(id.clone());
        self.flights.push(Flight {
            id,
            mode: request.mode,
            waiters: vec![request],
        });
    }
}
pub(super) struct Cache {
    slots: VecDeque<Slot>,
}
impl Cache {
    pub(super) fn new() -> Self {
        Self {
            slots: VecDeque::new(),
        }
    }
    pub(super) fn slot(&mut self, key: &Key) -> Option<&mut Slot> {
        // Moving a hit to the back makes idle eviction least recently used.
        if let Some(index) = self.slots.iter().position(|slot| slot.key == *key) {
            let slot = self.slots.remove(index)?;
            self.slots.push_back(slot);
        } else {
            if self.slots.len() >= MAX_KEYS {
                let idle = self.slots.iter().position(|slot| slot.flights.is_empty())?;
                let _evicted = self.slots.remove(idle);
            }
            self.slots.push_back(Slot {
                key: key.clone(),
                current: None,
                cached: None,
                flights: Vec::new(),
            });
        }
        self.slots.back_mut()
    }
    pub(super) fn finish(&mut self, key: &Key, id: &LoadId, result: Outcome, finished: Instant) {
        let Some(slot) = self.slots.iter_mut().find(|slot| slot.key == *key) else {
            return;
        };
        let Some(index) = slot.flights.iter().position(|flight| flight.id.same(id)) else {
            return;
        };
        let flight = slot.flights.remove(index);
        if slot
            .current
            .as_ref()
            .is_some_and(|current| current.same(id))
        {
            slot.cached = result.as_ref().ok().map(|page| Cached {
                page: Arc::clone(page),
                finished,
            });
        }
        for waiter in flight.waiters {
            waiter.answer(result.clone());
        }
    }
    pub(super) fn stop(&mut self) {
        for slot in self.slots.drain(..) {
            for flight in slot.flights {
                for waiter in flight.waiters {
                    waiter.answer(Err(Arc::new(Error::Stopped)));
                }
            }
        }
    }
}
