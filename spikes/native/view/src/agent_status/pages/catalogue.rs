//! Registry reads share the machine page's existing ticket and cache lifecycle.
use super::{MachineKey, Operation, Page, Pages, RequestId};
use crate::agent_status::catalogue::Read;
use vessel_proto::{NodeId, agents::Catalogue};

impl<E> Page<E> {
    /// Describe only this inventory's node-accepted catalog. A failure or
    /// refresh retains the fetched copy; another operation does not borrow
    /// its error as a registry failure.
    #[must_use]
    pub fn catalogue(&self) -> Read<'_, E> {
        let previous = self.catalogue.as_ref().and_then(|(node, copy)| {
            self.inventory
                .as_ref()
                .filter(|snapshot| snapshot.node == *node)
                .map(|_| copy)
        });
        if self
            .pending
            .as_ref()
            .is_some_and(|ticket| matches!(ticket.operation, Operation::Catalogue { .. }))
        {
            return Read::Pending { previous };
        }
        if let Some(issue) = &self.issue
            && matches!(issue.operation, Operation::Catalogue { .. })
        {
            return Read::Failed {
                error: &issue.source,
                previous,
            };
        }
        previous.map_or(Read::NotRequested, Read::Ready)
    }
}
impl<E> Pages<E> {
    /// Accept only the matching registry operation and its physical node;
    /// another read, late reply or reused book name cannot seed this cache.
    #[must_use]
    pub fn catalogue_loaded(
        &mut self,
        machine: &MachineKey,
        id: RequestId,
        node: NodeId,
        catalogue: Catalogue,
    ) -> bool {
        let Some(page) = self.pending(machine, id) else {
            return false;
        };
        if !matches!(
            page.pending.as_ref().map(|ticket| &ticket.operation),
            Some(Operation::Catalogue { node: expected }) if *expected == node
        ) || page
            .inventory
            .as_ref()
            .is_none_or(|snapshot| snapshot.node != node)
        {
            return false;
        }
        page.catalogue = Some((node, catalogue));
        page.pending = None;
        true
    }
}

#[cfg(test)]
mod tests;
