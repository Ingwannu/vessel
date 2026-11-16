//! Retained assistant image acquisition: one flight per key and a soft LRU cap.
//! The cache does no I/O. Keys are the host's canonical node/occurrence/file identity;
//! values own any attachment pin/preview resource so eviction drops it naturally.

use std::collections::{HashMap, HashSet, VecDeque, hash_map::Entry};
use std::hash::Hash;
use std::sync::Arc;

/// Reference acquisition-cache capacity; retained consumers can temporarily exceed it.
pub const IMAGE_CACHE_CAPACITY: usize = 500;

/// An acquisition result cannot settle another generation of the same image key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AcquisitionId(u64);

/// Each mounted consumer releases its own retention, idempotently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConsumerId(u64);

/// Whether a caller keeps the image mounted or merely prefetches it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retention {
    /// A visible consumer prevents LRU eviction until released.
    Retained,
    /// Prefetch still receives its result, even if the cache evicts its entry.
    Transient,
}

/// A second consumer joins the same flight instead of issuing another read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Acquisition<Value> {
    /// The host starts one bounded, generation-fenced read.
    Start(AcquisitionId),
    /// A previous read owns the result.
    Pending(AcquisitionId),
    /// An already acquired resource can be displayed immediately.
    Ready(Arc<Value>),
}

/// Retention and the read action are returned together so neither is forgotten.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct Acquired<Value> {
    /// None for a transient prefetch; retained views release this token on unmount.
    pub consumer: Option<ConsumerId>,
    /// Start/join a read or display its cached resource.
    pub acquisition: Acquisition<Value>,
}

/// Even an evicted flight still resolves to the waiting caller, as in the reference.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub enum Completion<Value, Failure> {
    /// Stored and returned; cheap shared values retain their resource owner.
    Stored(Arc<Value>),
    /// No longer cached; the caller checks its own current workspace generation.
    Uncached(Arc<Value>),
    /// Failure removes the cache entry so a later acquisition can retry.
    Failed(Failure),
}

/// Releasing an already released/failed consumer has no side effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Released {
    /// The consumer's retention ended.
    Released,
    /// It was already released or its failed acquisition removed it.
    AlreadyReleased,
}

/// The cache never reuses a correlation identity after counter exhaustion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CacheError {
    /// No mutation/read is admitted with a reused correlation token.
    #[error("image acquisition identities are exhausted")]
    IdentityExhausted,
}

/// A count retains its unit at the host boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryCount(usize);
impl EntryCount {
    /// Number of currently cached acquisitions, including pending ones.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

#[derive(Debug)]
enum State<Value> {
    Pending(AcquisitionId),
    Ready(Arc<Value>),
}
#[derive(Debug)]
struct ImageEntry<Value> {
    state: State<Value>,
    consumers: HashSet<ConsumerId>,
}

/// An application actor owns this cache; no toolkit, timer or shared mutable task state.
#[derive(Debug)]
pub struct ImageCache<Key, Value> {
    entries: HashMap<Key, ImageEntry<Value>>,
    order: VecDeque<Key>,
    pending: HashMap<AcquisitionId, Key>,
    consumers: HashMap<ConsumerId, Key>,
    next: u64,
}

impl<Key, Value> Default for ImageCache<Key, Value> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            pending: HashMap::new(),
            consumers: HashMap::new(),
            next: 0,
        }
    }
}

impl<Key: Clone + Eq + Hash, Value> ImageCache<Key, Value> {
    /// Only new keys start a read; all consumer identities are reserved before mutation.
    ///
    /// # Errors
    /// Identity exhaustion leaves the cache contents unchanged.
    pub fn acquire(
        &mut self,
        key: Key,
        retention: Retention,
    ) -> Result<Acquired<Value>, CacheError> {
        let consumer = match retention {
            Retention::Retained => Some(ConsumerId(identity(&mut self.next)?)),
            Retention::Transient => None,
        };
        let acquisition = match self.entries.entry(key.clone()) {
            Entry::Occupied(mut entry) => {
                if let Some(consumer) = consumer {
                    let _new = entry.get_mut().consumers.insert(consumer);
                }
                match &entry.get().state {
                    State::Pending(request) => Acquisition::Pending(*request),
                    State::Ready(value) => Acquisition::Ready(Arc::clone(value)),
                }
            }
            Entry::Vacant(entry) => {
                let request = AcquisitionId(identity(&mut self.next)?);
                let _entry = entry.insert(ImageEntry {
                    state: State::Pending(request),
                    consumers: consumer.into_iter().collect(),
                });
                let _old = self.pending.insert(request, key.clone());
                Acquisition::Start(request)
            }
        };
        self.touch(&key);
        if let Some(consumer) = consumer {
            let _old = self.consumers.insert(consumer, key);
        }
        self.enforce_capacity();
        Ok(Acquired {
            consumer,
            acquisition,
        })
    }

    /// A completion keeps its value for the caller even after eviction; stale keys
    /// cannot overwrite a newer acquisition because request identity is independent.
    pub fn complete<Failure>(
        &mut self,
        request: AcquisitionId,
        result: Result<Arc<Value>, Failure>,
    ) -> Completion<Value, Failure> {
        let key = self.pending.remove(&request);
        match (key, result) {
            (Some(key), Ok(value)) => {
                if let Some(entry) = self.entries.get_mut(&key) {
                    entry.state = State::Ready(Arc::clone(&value));
                    Completion::Stored(value)
                } else {
                    Completion::Uncached(value)
                }
            }
            (None, Ok(value)) => Completion::Uncached(value),
            (key, Err(error)) => {
                if let Some(key) = key {
                    self.evict(&key);
                }
                Completion::Failed(error)
            }
        }
    }

    /// Only resolved entries count as a peek hit, and a hit refreshes LRU position.
    pub fn peek(&mut self, key: &Key) -> Option<Arc<Value>> {
        let value = match &self.entries.get(key)?.state {
            State::Pending(_) => return None,
            State::Ready(value) => Arc::clone(value),
        };
        self.touch(key);
        Some(value)
    }

    /// A visible consumer's lifetime ends independently of the read's completion.
    pub fn release(&mut self, consumer: ConsumerId) -> Released {
        let Some(key) = self.consumers.remove(&consumer) else {
            return Released::AlreadyReleased;
        };
        if let Some(entry) = self.entries.get_mut(&key) {
            let _gone = entry.consumers.remove(&consumer);
        }
        self.enforce_capacity();
        Released::Released
    }

    /// Counts include retained overflow, matching the reference's soft capacity.
    #[must_use]
    pub fn count(&self) -> EntryCount {
        EntryCount(self.entries.len())
    }

    fn touch(&mut self, key: &Key) {
        self.order.retain(|other| other != key);
        self.order.push_back(key.clone());
    }

    fn enforce_capacity(&mut self) {
        while self.entries.len() > IMAGE_CACHE_CAPACITY {
            let idle = self
                .order
                .iter()
                .find(|key| {
                    self.entries
                        .get(*key)
                        .is_some_and(|entry| entry.consumers.is_empty())
                })
                .cloned();
            let Some(idle) = idle else {
                return;
            };
            self.evict(&idle);
        }
    }

    fn evict(&mut self, key: &Key) {
        if let Some(entry) = self.entries.remove(key) {
            for consumer in entry.consumers {
                let _gone = self.consumers.remove(&consumer);
            }
            match entry.state {
                State::Pending(request) => {
                    let _gone = self.pending.remove(&request);
                }
                State::Ready(_value) => {}
            }
        }
        self.order.retain(|other| other != key);
    }
}

fn identity(next: &mut u64) -> Result<u64, CacheError> {
    let issued = next.checked_add(1).ok_or(CacheError::IdentityExhausted)?;
    *next = issued;
    Ok(issued)
}

#[cfg(test)]
mod tests;
