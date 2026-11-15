//! One per-shell catalog owner and bounded read scheduler (ADR0079).
//!
//! This stays under the parent module's staging gate until the real Node SDK
//! path exists. It uses the one canonical catalog, not another fake wire model.
//! Verified NodeId/support comes from the shared connection owner; no alias or
//! cwd fallback can manufacture it. Feature projections run after accept returns,
//! outside any registry borrow and outside paint.
#[cfg(test)]
mod tests;

use super::{Opener, SurfaceView};
use std::{
    cell::{Ref, RefCell},
    collections::{BTreeSet, VecDeque},
    rc::Rc,
};
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::plugins::{Command, Event};
use vessel_app_view::plugins::{
    CatalogTicket, HostSupport, Problem, Registry, SnapshotApplied, SurfaceProblem,
};
use vessel_proto::{NodeId, plugins::ContributionAddress};

/// Bound both active source futures and undelivered complete replies per Hub.
/// The link's finite deadline remains independent from this count admission.
const MAX_CONCURRENT_CATALOG_READS: usize = 4;

/// Notifications contain no declaration/diagnostic payload. Their owner updates
/// theme/commands/sidebar/panels from the one registry after mutation settles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Change {
    Catalog(NodeId),
    Eligibility(NodeId),
    Failure { node: NodeId, problem: Problem },
}

/// A busy owner returns the still-owned completion instead of silently losing
/// a reply and leaking its read slot. The caller defers this same event.
#[must_use]
pub(super) enum Acceptance {
    Settled(Option<Change>),
    Deferred(Event),
}

pub(super) struct Hub {
    registry: Rc<RefCell<Registry>>,
    queued: VecDeque<CatalogTicket>,
    active: Vec<CatalogTicket>,
    dirty: BTreeSet<NodeId>,
}

impl Hub {
    pub(super) fn new() -> Self {
        Self {
            registry: Rc::new(RefCell::new(Registry::default())),
            queued: VecDeque::new(),
            active: Vec::new(),
            dirty: BTreeSet::new(),
        }
    }

    /// Only the connection owner's verified welcome calls this. Reconnect
    /// invalidates old publication, but an already-running read retains its slot.
    pub(super) fn connected(
        &mut self,
        node: NodeId,
        support: HostSupport,
    ) -> Result<Change, Problem> {
        let result = {
            let mut registry = self
                .registry
                .try_borrow_mut()
                .map_err(|_busy| Problem::Busy)?;
            registry.offline(node);
            registry.begin(node, support)
        };
        self.queued.retain(|ticket| ticket.node() != node);
        let _was_dirty = self.dirty.remove(&node);
        Ok(match result {
            Ok(ticket) => {
                self.queued.push_back(ticket);
                Change::Eligibility(node)
            }
            // Unsupported is a connection fact which withdrew cached data;
            // it must reach projections rather than be skipped by `?`.
            Err(problem) => Change::Failure { node, problem },
        })
    }

    /// One pending bit coalesces any number of invalidations for an admitted
    /// online host. The registry's host/one-pending-read bound also bounds this queue.
    pub(super) fn invalidate(&mut self, node: NodeId) -> Result<(), Problem> {
        let result = {
            let mut registry = self
                .registry
                .try_borrow_mut()
                .map_err(|_busy| Problem::Busy)?;
            registry.can_refresh(node)?;
            // A not-yet-dispatched read will observe the latest invalidation.
            if self.queued.iter().any(|ticket| ticket.node() == node) {
                return Ok(());
            }
            registry.begin(node, HostSupport::Supported)
        };
        match result {
            Ok(ticket) => self.queued.push_back(ticket),
            Err(Problem::Busy) => {
                let _already_dirty = self.dirty.insert(node);
            }
            Err(problem) => return Err(problem),
        }
        Ok(())
    }

    /// The caller dispatches exactly these commands through the one link path.
    pub(super) fn take_reads(&mut self) -> Vec<Command> {
        let mut commands = Vec::new();
        while self.active.len() < MAX_CONCURRENT_CATALOG_READS {
            let Some(ticket) = self.queued.pop_front() else {
                break;
            };
            self.active.push(ticket);
            commands.push(Command::ReadCatalog(ticket));
        }
        commands
    }

    /// Retirement releases a slot exactly once, including a now-stale read.
    /// Disconnect/forget never release it early while a source still owns work.
    pub(super) fn accept(&mut self, event: Event) -> Acceptance {
        let Event::CatalogRead { ticket, .. } = &event;
        let Some(index) = self.active.iter().position(|active| active == ticket) else {
            return Acceptance::Settled(None);
        };
        // Acquire before retiring: a reentrant reader must not lose completion
        // ownership without applying it. The event caller may retry on Busy.
        let applied = {
            let Ok(mut registry) = self.registry.try_borrow_mut() else {
                return Acceptance::Deferred(event);
            };
            let Event::CatalogRead { ticket, result } = event;
            registry.apply(ticket, result)
        };
        let ticket = self.active[index];
        let _retired = self.active.swap_remove(index);
        let node = ticket.node();
        if self.dirty.remove(&node) {
            match self.invalidate(node) {
                Ok(()) | Err(Problem::Offline | Problem::Stale | Problem::Unsupported) => {}
                Err(problem) => {
                    return Acceptance::Settled(Some(Change::Failure { node, problem }));
                }
            }
        }
        Acceptance::Settled(match applied {
            Ok(SnapshotApplied::Applied) => Some(Change::Catalog(node)),
            Ok(SnapshotApplied::Stale) => None,
            Err(problem) => Some(Change::Failure { node, problem }),
        })
    }

    pub(super) fn offline(&mut self, node: NodeId) -> Result<Change, Problem> {
        self.registry
            .try_borrow_mut()
            .map_err(|_busy| Problem::Busy)?
            .offline(node);
        self.queued.retain(|ticket| ticket.node() != node);
        let _was_dirty = self.dirty.remove(&node);
        Ok(Change::Eligibility(node))
    }

    pub(super) fn forget(&mut self, node: NodeId) -> Result<Change, Problem> {
        self.registry
            .try_borrow_mut()
            .map_err(|_busy| Problem::Busy)?
            .remove(node);
        self.queued.retain(|ticket| ticket.node() != node);
        let _was_dirty = self.dirty.remove(&node);
        Ok(Change::Eligibility(node))
    }

    pub(super) fn registry(&self) -> Result<Ref<'_, Registry>, Problem> {
        self.registry.try_borrow().map_err(|_busy| Problem::Busy)
    }

    pub(super) fn surface(
        &self,
        address: ContributionAddress,
        opener: Opener,
        window: &mut Window,
        cx: &mut Context<'_, SurfaceView>,
    ) -> Result<SurfaceView, SurfaceProblem> {
        SurfaceView::new(&self.registry, address, opener, window, cx)
    }
}
