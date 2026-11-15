//! Applying a reply requires its still-pending machine and ticket.
//! Installation output remains attached to that page, never the visible one.

use super::{
    AgentReadiness, Completion, InstallJob, InstallLog, InstallOutput, InstallProgress, Inventory,
    Issue, MachineKey, Operation, Pages, RemovalFreshness, RequestId, Transcript,
};

impl<E> Pages<E> {
    /// An actual connection invalidates old reads even for the same node id.
    /// Preserve install output only when the page's retained identity proves
    /// it belongs to this same physical node, never a reused book name.
    #[must_use]
    pub fn connected(&mut self, machine: &MachineKey, node: vessel_proto::NodeId) -> bool {
        let Some(page) = self.entries.get_mut(machine.as_str()) else {
            return false;
        };
        let same_node = page.node == Some(node);
        let selected = if same_node {
            page.selected.take()
        } else {
            None
        };
        let install = if same_node {
            match std::mem::take(&mut page.install) {
                InstallProgress::Following(log) => InstallProgress::Unavailable(log),
                previous @ (InstallProgress::None
                | InstallProgress::Finished { .. }
                | InstallProgress::Unavailable(_)) => previous,
            }
        } else {
            InstallProgress::None
        };
        // The sequence belongs to Pages and intentionally survives this reset.
        *page = super::Page {
            node: Some(node),
            install,
            selected,
            ..super::Page::default()
        };
        true
    }

    /// Keep the facts visible but require a successful discovery before another
    /// removal. An interrupted mutation may already have changed the node.
    #[must_use]
    pub fn removal_uncertain(&mut self, machine: &MachineKey, id: RequestId, source: E) -> bool {
        let Some(page) = self.pending(machine, id) else {
            return false;
        };
        if !matches!(
            page.pending.as_ref().map(|ticket| &ticket.operation),
            Some(Operation::Remove { .. })
        ) {
            return false;
        }
        page.removal = RemovalFreshness::RefreshRequired;
        self.failed(machine, id, source);
        true
    }

    /// Forget only the acknowledged definition on its own pending page.
    /// A failure instead retains the entire snapshot through [`Self::failed`].
    #[must_use]
    pub fn removed(
        &mut self,
        machine: &MachineKey,
        id: RequestId,
        node: vessel_proto::NodeId,
        name: &vessel_proto::agents::AgentName,
    ) -> bool {
        let Some(page) = self.pending(machine, id) else {
            return false;
        };
        if !matches!(
            page.pending.as_ref().map(|ticket| &ticket.operation),
            Some(Operation::Remove { node: expected, name: target })
                if *expected == node && target == name
        ) || page
            .inventory
            .as_ref()
            .is_none_or(|snapshot| snapshot.node != node)
        {
            return false;
        }
        let agent = vessel_proto::HarnessRef::Added(name.clone());
        if let Some(inventory) = &mut page.inventory {
            inventory.agents.retain(|info| info.agent != agent);
            inventory.harnesses.retain(|found| found.kind != agent);
            inventory.readiness.retain(|ready| ready.agent != agent);
        }
        if page.selected.as_ref() == Some(&agent) {
            page.selected = None;
        }
        page.pending = None;
        true
    }

    /// A snapshot answers only its own discovery request. The caller may start
    /// follow-up work only when this transition accepts that original reply.
    #[must_use]
    pub fn loaded(&mut self, machine: &MachineKey, id: RequestId, inventory: Inventory) -> bool {
        if let Some(page) = self.pending(machine, id)
            && matches!(
                page.pending.as_ref().map(|ticket| &ticket.operation),
                Some(Operation::Discovery)
            )
        {
            // A fresh inventory can establish another client's removal too.
            // Same-node reconnect preserves navigation only while it still
            // names a definition the machine reports.
            if page.selected.as_ref().is_some_and(|selected| {
                !inventory.agents.iter().any(|info| &info.agent == selected)
                    && !inventory
                        .harnesses
                        .iter()
                        .any(|found| &found.kind == selected)
            }) {
                page.selected = None;
            }
            // A later book-name reuse must not resurrect an older node's
            // registry if the book eventually points back to that id (ABA).
            if page
                .catalogue
                .as_ref()
                .is_some_and(|(node, _)| *node != inventory.node)
            {
                page.catalogue = None;
            }
            page.node = Some(inventory.node);
            page.inventory = Some(inventory);
            page.pending = None;
            page.removal = RemovalFreshness::Current;
            return true;
        }
        false
    }
    /// Apply a check by agent/profile identity, never by the visible selection.
    pub fn checked(&mut self, machine: &MachineKey, id: RequestId, checked: Vec<AgentReadiness>) {
        if let Some(page) = self.pending(machine, id)
            && matches!(
                page.pending.as_ref().map(|ticket| &ticket.operation),
                Some(Operation::Check(_))
            )
        {
            if let Some(inventory) = &mut page.inventory {
                for ready in checked {
                    inventory
                        .readiness
                        .retain(|old| old.agent != ready.agent || old.profile != ready.profile);
                    inventory.readiness.push(ready);
                }
            }
            page.pending = None;
        }
    }
    /// The node accepted this page's install request.
    pub fn installing(&mut self, machine: &MachineKey, id: RequestId, job: InstallJob) {
        if let Some(page) = self.pending(machine, id)
            && matches!(
                page.pending.as_ref().map(|ticket| &ticket.operation),
                Some(Operation::Install(_))
            )
        {
            page.install = InstallProgress::Following(InstallLog {
                job,
                transcript: Transcript::default(),
            });
        }
    }
    /// A page belongs to its install even while another machine is shown.
    pub fn output(&mut self, machine: &MachineKey, id: RequestId, output: &InstallOutput) {
        if let Some(page) = self.pending(machine, id)
            && let InstallProgress::Following(log) = &mut page.install
        {
            log.transcript.append(&output.bytes);
            if output.truncated {
                log.transcript.clipped_on_node();
            }
        }
    }
    /// Completion is accepted only after the link drained all pages.
    #[must_use]
    pub fn finished(
        &mut self,
        machine: &MachineKey,
        id: RequestId,
        completion: Completion,
    ) -> bool {
        if let Some(page) = self.pending(machine, id)
            && let InstallProgress::Following(log) = std::mem::take(&mut page.install)
        {
            page.install = InstallProgress::Finished { log, completion };
            page.pending = None;
            page.inventory = None;
            return true;
        }
        false
    }
    /// Preserve the error and stop claiming the output is still being followed.
    pub fn failed(&mut self, machine: &MachineKey, id: RequestId, source: E) {
        if let Some(page) = self.pending(machine, id)
            && let Some(ticket) = page.pending.take()
        {
            page.install = match std::mem::take(&mut page.install) {
                InstallProgress::Following(log) => InstallProgress::Unavailable(log),
                status @ (InstallProgress::None
                | InstallProgress::Finished { .. }
                | InstallProgress::Unavailable(_)) => status,
            };
            page.issue = Some(Issue {
                operation: ticket.operation,
                source,
            });
        }
    }
}
