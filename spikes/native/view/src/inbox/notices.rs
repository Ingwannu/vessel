//! A backgrounded window may not draw for hours. Coalescing bounds pending OS
//! delivery without losing durable inbox entries or replaying them on reload.
use super::Key;
use std::collections::VecDeque;

/// At most this many individual sources wait for the next window update.
pub const MAX_PENDING_NOTICES: usize = 128;

/// Whether delivery must explain that several alerts were combined.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Coalescing {
    /// Every pending source is represented individually.
    #[default]
    Individual,
    /// Show an in-app coalescing notice; OS delivery still needs each source claim.
    Combined,
}

/// A bounded queue of delivery opportunities, not a second attention store.
#[derive(Debug, Default)]
pub struct NoticeQueue {
    keys: VecDeque<Key>,
    coalescing: Coalescing,
}
impl NoticeQueue {
    /// Overflow coalesces explicitly rather than allocating while minimized.
    pub fn extend(&mut self, keys: impl IntoIterator<Item = Key>) {
        for key in keys {
            if self.keys.contains(&key) {
                continue;
            }
            if self.keys.len() == MAX_PENDING_NOTICES {
                let _oldest = self.keys.pop_front();
                self.coalescing = Coalescing::Combined;
            }
            self.keys.push_back(key);
        }
    }
    /// Drop opportunities whose durable source has already disappeared.
    pub fn retain(&mut self, keep: impl FnMut(&Key) -> bool) {
        self.keys.retain(keep);
    }
    /// Drain once; delivery failure never changes any node's read state.
    #[must_use]
    pub fn take(&mut self) -> (Vec<Key>, Coalescing) {
        (
            self.keys.drain(..).collect(),
            std::mem::take(&mut self.coalescing),
        )
    }
}

/// Random opaque notification identifier generated at the app's OS boundary;
/// source identifiers never enter the operating system's notification payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeliveryTag([u8; 16]);
impl DeliveryTag {
    /// UUID bytes from the existing OS-boundary random generator.
    #[must_use]
    pub const fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
    /// Encode only this unrelated opaque identity for the OS callback.
    #[must_use]
    pub const fn bytes(self) -> [u8; 16] {
        self.0
    }
}

/// Local-only destination; OS callbacks carry only the unrelated random tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    /// Exact session inbox entry.
    Session(Key),
    /// Exact node terminal, with no invented session relationship.
    Terminal(super::terminals::Key),
}
impl From<Key> for Destination {
    fn from(key: Key) -> Self {
        Self::Session(key)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overflow_is_bounded_and_explicit_and_a_batch_can_only_be_consumed_once() {
        let node = vessel_proto::NodeId::random();
        let session = vessel_proto::SessionId::random();
        let mut queue = NoticeQueue::default();
        let beyond = MAX_PENDING_NOTICES.checked_add(1).unwrap();
        queue.extend((0..beyond).map(|seq| Key {
            node,
            session,
            seq: vessel_proto::EventSeq::new(u64::try_from(seq).unwrap()),
        }));
        let (keys, coalescing) = queue.take();
        assert_eq!(keys.len(), MAX_PENDING_NOTICES);
        assert_eq!(coalescing, Coalescing::Combined);
        assert_eq!(queue.take(), (Vec::new(), Coalescing::Individual));
        queue.extend(keys);
        queue.retain(|_key| false);
        assert_eq!(queue.take(), (Vec::new(), Coalescing::Individual));
    }
}
