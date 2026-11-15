//! Bounded physical markers follow grid edits, without per-row identity allocation.
use alloc::vec::Vec;

/// Per-buffer ceiling; retired handles still consume their caller-owned slot.
pub const MAX_MARKERS: usize = 64;

/// A grid-local allocation, meaningful only with the facade's buffer/reset identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Marker(u64);
/// Missing/released markers are distinct from rows genuinely removed from history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerPosition {
    /// Absolute physical row in the retained buffer.
    Retained(usize),
    /// The row was removed; the caller can explicitly clamp and replace.
    Evicted,
    /// Its caller released or replaced the handle.
    Released,
}
/// Allocation refuses before changing an existing valid marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerError {
    /// Requested row is not retained.
    Row,
    /// All finite marker slots are caller-owned.
    Capacity,
    /// This table cannot allocate another non-reused identity.
    Exhausted,
    /// A replacement names an already released handle.
    Released,
}
/// Release is idempotent without admitting an unrelated handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerRelease {
    /// Existing marker retired.
    Released,
    /// No entry with this identity remained.
    AlreadyGone,
}
#[derive(Debug, Clone)]
struct Entry {
    id: Marker,
    row: Option<usize>,
}
#[derive(Debug, Clone, Default)]
pub(crate) struct Markers {
    next: u64,
    entries: Vec<Entry>,
}
impl Markers {
    fn identity(&mut self) -> Result<Marker, MarkerError> {
        let next = self.next.checked_add(1).ok_or(MarkerError::Exhausted)?;
        self.next = next;
        Ok(Marker(next))
    }
    pub(crate) fn create(
        &mut self,
        row: usize,
    ) -> Result<Marker, MarkerError> {
        if self.entries.len() >= MAX_MARKERS
            || self.entries.try_reserve(1).is_err()
        {
            return Err(MarkerError::Capacity);
        }
        let id = self.identity()?;
        self.entries.push(Entry { id, row: Some(row) });
        Ok(id)
    }
    pub(crate) fn replace(
        &mut self,
        old: Marker,
        row: usize,
    ) -> Result<Marker, MarkerError> {
        let at = self
            .entries
            .iter()
            .position(|entry| entry.id == old)
            .ok_or(MarkerError::Released)?;
        let id = self.identity()?;
        let entry = self.entries.get_mut(at).ok_or(MarkerError::Released)?;
        *entry = Entry { id, row: Some(row) };
        Ok(id)
    }
    pub(crate) fn resolve(&self, id: Marker) -> MarkerPosition {
        match self.entries.iter().find(|entry| entry.id == id) {
            Some(Entry { row: Some(row), .. }) => {
                MarkerPosition::Retained(*row)
            }
            Some(Entry { row: None, .. }) => MarkerPosition::Evicted,
            None => MarkerPosition::Released,
        }
    }
    pub(crate) fn release(&mut self, id: Marker) -> MarkerRelease {
        match self.entries.iter().position(|entry| entry.id == id) {
            Some(at) => {
                drop(self.entries.swap_remove(at));
                MarkerRelease::Released
            }
            None => MarkerRelease::AlreadyGone,
        }
    }
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }
    pub(crate) fn trim(&mut self, count: usize) {
        self.delete(0, count);
    }
    pub(crate) fn insert(&mut self, at: usize, count: usize) {
        for entry in &mut self.entries {
            entry.row = entry.row.map(|row| {
                if row >= at {
                    row.saturating_add(count)
                } else {
                    row
                }
            });
        }
    }
    pub(crate) fn delete(&mut self, at: usize, count: usize) {
        let end = at.saturating_add(count);
        for entry in &mut self.entries {
            entry.row = entry.row.and_then(|row| {
                if row < at {
                    Some(row)
                } else if row < end {
                    None
                } else {
                    row.checked_sub(count)
                }
            });
        }
    }
    pub(crate) fn remap(&mut self, mapping: &[Option<usize>]) {
        for entry in &mut self.entries {
            entry.row = entry
                .row
                .and_then(|row| mapping.get(row).copied().flatten());
        }
    }
}
