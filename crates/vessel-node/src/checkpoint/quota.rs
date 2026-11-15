//! Reserved bytes include obsolete work until its cleanup completes.

use vessel_proto::checkpoint::{CHECKPOINT_NODE_BYTES, CheckpointError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ReservedBytes(u64);
impl ReservedBytes {
    pub(super) const fn capture() -> Self {
        Self(vessel_proto::checkpoint::CHECKPOINT_CAPTURE_RESERVATION_BYTES)
    }
    pub(super) const fn get(self) -> u64 {
        self.0
    }
    #[cfg(test)]
    pub(super) fn new(bytes: u64) -> Result<Self, CheckpointError> {
        if bytes > CHECKPOINT_NODE_BYTES {
            Err(CheckpointError::StorageFull)
        } else {
            Ok(Self(bytes))
        }
    }
}
#[derive(Debug, Default)]
pub(super) struct Quota {
    reserved: u64,
}
impl Quota {
    pub(super) fn reserve(&mut self, bytes: ReservedBytes) -> Result<(), CheckpointError> {
        let next = self
            .reserved
            .checked_add(bytes.get())
            .ok_or(CheckpointError::StorageFull)?;
        if next > CHECKPOINT_NODE_BYTES {
            return Err(CheckpointError::StorageFull);
        }
        self.reserved = next;
        Ok(())
    }
    pub(super) fn release(&mut self, bytes: ReservedBytes) {
        // Releases are for a removed job exactly once, so underflow is a bug,
        // but it cannot wrap a reservation into more available capacity.
        self.reserved = self.reserved.saturating_sub(bytes.get());
    }
    #[cfg(test)]
    pub(super) fn reserved(&self) -> u64 {
        self.reserved
    }
}
