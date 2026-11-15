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
/// Bounded, process-local open routing. Old notifications after a restart or
/// eviction open the inbox generically rather than the wrong conversation.
#[derive(Debug, Default)]
pub struct NoticeRoutes(VecDeque<(DeliveryTag, Destination)>);
impl NoticeRoutes {
    /// Keep the source locally; it must never be serialized into the OS tag.
    pub fn remember(&mut self, tag: DeliveryTag, key: impl Into<Destination>) {
        let key = key.into();
        self.0.retain(|(known, _)| *known != tag);
        if self.0.len() == MAX_PENDING_NOTICES {
            let _oldest = self.0.pop_front();
        }
        self.0.push_back((tag, key));
    }
    /// Activation is consumed once, including duplicate callback delivery.
    #[must_use]
    pub fn take(&mut self, tag: DeliveryTag) -> Option<Destination> {
        let index = self.0.iter().position(|(known, _)| *known == tag)?;
        self.0.remove(index).map(|(_, key)| key)
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
    #[test]
    fn opaque_open_routes_are_bounded_consumed_once_and_do_not_guess_after_restart() {
        let key = Key {
            node: vessel_proto::NodeId::random(),
            session: vessel_proto::SessionId::random(),
            seq: vessel_proto::EventSeq::new(1),
        };
        let tag = DeliveryTag::new([1; 16]);
        let mut routes = NoticeRoutes::default();
        routes.remember(tag, key);
        assert_eq!(routes.take(tag), Some(Destination::Session(key)));
        assert_eq!(routes.take(tag), None);
        assert_eq!(NoticeRoutes::default().take(tag), None);
        let total = MAX_PENDING_NOTICES.checked_add(1).unwrap();
        for index in 0..total {
            let mut bytes = [0; 16];
            let encoded = u64::try_from(index).unwrap().to_le_bytes();
            bytes
                .get_mut(..encoded.len())
                .unwrap()
                .copy_from_slice(&encoded);
            routes.remember(DeliveryTag::new(bytes), key);
        }
        assert_eq!(routes.0.len(), MAX_PENDING_NOTICES);
        assert_eq!(routes.take(DeliveryTag::new([0; 16])), None);
    }
}
