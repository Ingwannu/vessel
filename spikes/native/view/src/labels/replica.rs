//! Apply catalog snapshots and contiguous updates without inventing deletions.
//! Offline/failed caches remain readable but cannot authorize mutations.

use vessel_proto::workspace_labels::{
    CatalogCursor, CatalogDelta, CatalogList, CatalogMode, CatalogUpdate,
};

use super::{Catalog, Failure, HostState, Label};

/// One host's cache, cursor and authority. A gap forces a fresh snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replica {
    catalog: Catalog,
    cursor: Option<CatalogCursor>,
    state: HostState,
}

impl Default for Replica {
    fn default() -> Self {
        Self {
            catalog: Catalog::default(),
            cursor: None,
            state: HostState::Loading,
        }
    }
}

impl Replica {
    /// Last successfully applied host-local definitions, including while offline.
    #[must_use]
    pub const fn catalog(&self) -> &Catalog {
        &self.catalog
    }
    /// Cursor for reconnect catch-up; none after a sequence mismatch.
    #[must_use]
    pub const fn cursor(&self) -> Option<CatalogCursor> {
        self.cursor
    }
    /// Only Online means the cache is currently authoritative.
    #[must_use]
    pub const fn state(&self) -> HostState {
        self.state
    }
    /// Connection loss releases authority, not the last known colors.
    pub const fn disconnected(&mut self) {
        self.state = HostState::Offline;
    }
    /// A new observation must establish authority before mutations are enabled.
    pub const fn reconnecting(&mut self) {
        self.state = HostState::Loading;
    }
    /// A failed first fetch never proves a catalog definition was deleted.
    pub const fn failed(&mut self, failure: Failure) {
        self.state = HostState::Failed(failure);
    }

    /// Validate the entire batch before replacing the cached projection.
    /// # Errors
    /// A gap, wrong generation/base/head, or duplicate snapshot identity needs resync.
    pub fn listed(&mut self, list: &CatalogList) -> Result<(), Failure> {
        let result = match &list.data {
            CatalogMode::Snapshot { labels } => snapshot(labels),
            CatalogMode::Changes { from, updates } => {
                if self.cursor != Some(*from) {
                    return self.mismatch();
                }
                let mut next = self.catalog.clone();
                let mut cursor = *from;
                for update in updates {
                    if !follows(cursor, update.cursor) {
                        return self.mismatch();
                    }
                    apply(&mut next, &update.delta);
                    cursor = update.cursor;
                }
                if cursor != list.head {
                    return self.mismatch();
                }
                Ok(next)
            }
        };
        match result {
            Ok(catalog) => {
                self.catalog = catalog;
                self.cursor = Some(list.head);
                self.state = HostState::Online;
                Ok(())
            }
            Err(_invalid_snapshot) => self.mismatch(),
        }
    }

    /// A stale or skipped live update leaves the whole previous cache intact.
    /// # Errors
    /// Any noncontiguous identity forces a snapshot, including after a disconnect.
    pub fn updated(&mut self, update: &CatalogUpdate) -> Result<(), Failure> {
        if !self.state.connected()
            || !self
                .cursor
                .is_some_and(|cursor| follows(cursor, update.cursor))
        {
            return self.mismatch();
        }
        apply(&mut self.catalog, &update.delta);
        self.cursor = Some(update.cursor);
        self.state = HostState::Online;
        Ok(())
    }

    fn mismatch<T>(&mut self) -> Result<T, Failure> {
        self.cursor = None;
        self.state = HostState::Failed(Failure::Load);
        Err(Failure::Load)
    }
}

fn follows(previous: CatalogCursor, next: CatalogCursor) -> bool {
    previous.generation == next.generation && previous.after.next() == Some(next.after)
}

fn apply(catalog: &mut Catalog, delta: &CatalogDelta) {
    match delta {
        CatalogDelta::Upsert { label, previous } => {
            catalog.upsert(previous.as_ref(), label.clone());
        }
        CatalogDelta::Remove { name } => catalog.remove(name),
    }
}

fn snapshot(labels: &[Label]) -> Result<Catalog, Failure> {
    let mut catalog = Catalog::default();
    for label in labels {
        if catalog.get(&label.name).is_some() {
            return Err(Failure::Load);
        }
        catalog.upsert(None, label.clone());
    }
    Ok(catalog)
}

#[cfg(test)]
mod tests;
