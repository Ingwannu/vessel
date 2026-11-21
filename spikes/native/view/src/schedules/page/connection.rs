//! Schedule facts retain their original admission; this is not a connection
//! registry or allocator. Only the canonical Fleet can supply a new context.
use super::super::load::MachineKey;
use super::{Cache, Page};
use crate::{Fleet, connection::ConnectionContext};

impl Cache {
    fn invalidate(&mut self) {
        self.scopes = None;
        for row in self.rows.values_mut() {
            row.authority = super::super::authority::Authority::default();
            row.mutation.reconnected();
        }
        self.read.invalidate();
    }
    fn bind(&mut self, context: Option<ConnectionContext>) -> bool {
        if context
            .as_ref()
            .is_some_and(|candidate| Some(candidate.epoch()) == self.retired)
        {
            return false;
        }
        if self.connection == context {
            return false;
        }
        let other_node = context.as_ref().is_some_and(|context| {
            self.origin
                .as_ref()
                .is_some_and(|origin| origin.node() != context.node())
        });
        self.connection = context;
        self.scopes = None;
        if other_node {
            // A reused alias does not make another node's schedule id ours.
            self.read = super::read::Read::default();
            self.rows.clear();
            self.origin = None;
        } else {
            self.invalidate();
        }
        true
    }
}
impl Page {
    /// Synchronize canonical admissions after the book keys. A changed context
    /// needs a fresh read even when every legacy Fleet display fact is equal.
    /// Returns whether presentation or read demand changed.
    #[must_use]
    pub fn admissions(&mut self, fleet: &Fleet) -> bool {
        let mut changed = false;
        for machine in &fleet.machines {
            if let Some(cache) = self.caches.get_mut(machine.name.as_str()) {
                changed |= cache.bind(machine.connection_context());
            }
        }
        changed
    }
    /// Disable actions as soon as the actor retires this admission, before
    /// a replacement Fleet snapshot reaches the window.
    pub fn retired(&mut self, tag: &crate::connection::ConnectionTag) -> bool {
        self.caches
            .get_mut(tag.machine().as_str())
            .is_some_and(|cache| {
                if cache
                    .connection
                    .as_ref()
                    .is_some_and(|context| context.epoch() == tag.epoch())
                {
                    cache.retired = Some(tag.epoch());
                    cache.bind(None)
                } else {
                    false
                }
            })
    }

    /// A fresh physical connection retires the old admission before its Fleet
    /// arrives. Cached same-node facts may stay visible, but offer no actions.
    pub fn connected(&mut self, machine: &MachineKey) {
        if let Some(cache) = self.caches.get_mut(machine) {
            cache.connection = None;
            // A fresh connection is in progress; do not invent an Offline
            // refusal before its canonical snapshot can actually settle.
            cache.invalidate();
        }
    }
    /// Admit only the explicitly selected canonical context. This pure edge
    /// supports alternate renderers; it never searches aliases by `NodeId`.
    #[must_use]
    pub fn admitted(&mut self, context: ConnectionContext) -> bool {
        self.caches
            .get_mut(context.machine().as_str())
            .is_some_and(|cache| cache.bind(Some(context)))
    }
}
