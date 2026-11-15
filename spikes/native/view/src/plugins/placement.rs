//! Live placement is a borrowed capability, never restored execution authority.

use vessel_proto::plugins::{
    ContributionAddress, ContributionContent, PanelContext, PanelLocation, PluginPanelContext,
};

use super::{Problem, Registry};

/// A live workspace/agent panel declaration. Private borrowed fields prevent
/// callers from fabricating or retaining this policy across catalog mutation.
/// Layout persists only its target and must resolve again for open/drop/restore.
#[derive(Debug)]
pub struct ResolvedPluginPanel<'a> {
    address: &'a ContributionAddress,
    context: &'a PluginPanelContext,
    locations: &'a [PanelLocation],
}

impl ResolvedPluginPanel<'_> {
    /// Exact node, installation generation, contribution kind and identity.
    #[must_use]
    pub const fn address(&self) -> &ContributionAddress { self.address }

    /// The context that was checked, including the agent identity where present.
    #[must_use]
    pub const fn context(&self) -> &PluginPanelContext { self.context }

    /// Validated manifest locations; layout cannot broaden them using Target's
    /// generic host support. A rejected restore remains in its original slot.
    #[must_use]
    pub const fn locations(&self) -> &[PanelLocation] { self.locations }

    /// Placement is explicit; explorer-only declarations cannot open in workspace.
    #[must_use]
    pub fn supports(&self, location: PanelLocation) -> bool { self.locations.contains(&location) }
}

impl Registry {
    /// Obtain an immediately usable placement capability from current facts.
    /// Agent-to-workspace ownership remains the shared workspace owner's check;
    /// a File context is resolved by the file panel owner's separate selector.
    ///
    /// # Errors
    /// Refuses offline/stale/missing contributions and incompatible contexts.
    pub fn resolve_panel<'a>(
        &'a self,
        address: &'a ContributionAddress,
        context: &'a PluginPanelContext,
    ) -> Result<ResolvedPluginPanel<'a>, Problem> {
        let contribution = self.resolve_contribution(address)?;
        let ContributionContent::Panel { context: declared, locations, .. } = &contribution.content else {
            return Err(Problem::Context);
        };
        match (declared, context) {
            (PanelContext::Workspace, PluginPanelContext::Workspace {}) |
            (PanelContext::Agent, PluginPanelContext::Agent { .. }) =>
                Ok(ResolvedPluginPanel { address, context, locations }),
            (PanelContext::Workspace, PluginPanelContext::Agent { .. } | PluginPanelContext::File { .. }) |
            (PanelContext::Agent, PluginPanelContext::Workspace {} | PluginPanelContext::File { .. }) =>
                Err(Problem::Context),
        }
    }
}

#[cfg(test)]
#[path = "placement_tests.rs"]
mod tests;
