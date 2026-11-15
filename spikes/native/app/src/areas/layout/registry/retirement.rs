//! Staged synchronous owner invalidation in the sole panel registry. No resource
//! state, transport, event queue, generation allocator or alternate tab map lives here.
use crate::shell::Shell;
use std::collections::{BTreeMap, btree_map::Entry};
use vessel_app_kit::ui::Context;
use vessel_app_view::layout::{Layout, LayoutError, LayoutKey, TabId, Target};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Owner {
    Launcher,
    Draft,
    Agent,
    ProviderChild,
    Terminal,
    Browser,
    Git,
    Files,
    Scripts,
    Plugin,
    Setup,
}
fn owner(target: &Target) -> Owner {
    match target {
        Target::NewTab => Owner::Launcher,
        Target::Draft(_) => Owner::Draft,
        Target::Agent(_) => Owner::Agent,
        Target::ProviderSubagent { .. } => Owner::ProviderChild,
        Target::Terminal(_) => Owner::Terminal,
        Target::Browser(_) => Owner::Browser,
        Target::Changes | Target::WorkingDiff | Target::CommitDiff(_) | Target::PullRequest => {
            Owner::Git
        }
        Target::Files | Target::File { .. } => Owner::Files,
        Target::Scripts => Owner::Scripts,
        Target::Plugin { .. } => Owner::Plugin,
        Target::Setup(_) => Owner::Setup,
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reason {
    Retargeted,
    Closed,
    CanonicalReplacement,
    Unavailable,
}
/// Notification is not save/discard/close permission. Original payloads and sent
/// actor tasks remain with their owner; only its UI instance/tickets retire.
pub(crate) struct Notice {
    workspace: LayoutKey,
    tab: TabId,
    target: Target,
    reason: Reason,
}
impl Notice {
    pub(crate) fn workspace(&self) -> &LayoutKey {
        &self.workspace
    }
    pub(crate) const fn tab(&self) -> TabId {
        self.tab
    }
    pub(crate) const fn target(&self) -> &Target {
        &self.target
    }
    pub(crate) const fn reason(&self) -> Reason {
        self.reason
    }
}
pub(crate) type Hook = fn(&mut Shell, &Notice, &mut Context<'_, Shell>);
pub(crate) struct Owners(BTreeMap<Owner, Hook>);
impl Owners {
    pub(crate) fn new() -> Self {
        Self(BTreeMap::new())
    }
    fn register(&mut self, owner: Owner, hook: Hook) -> Result<(), LayoutError> {
        match self.0.entry(owner) {
            Entry::Vacant(slot) => {
                let _hook = slot.insert(hook);
                Ok(())
            }
            Entry::Occupied(_) => Err(LayoutError::InvalidState),
        }
    }
    fn hook(&self, target: &Target) -> Option<Hook> {
        self.0.get(&owner(target)).copied()
    }
}
#[derive(Clone, Copy)]
pub(crate) enum Source {
    Local,
    Canonical,
    Unavailable,
}
/// Temporary metadata snapshot for one transaction, never retained as a second
/// book. Its notification collection is bounded by its already admitted tabs.
pub(crate) struct Before(Vec<(LayoutKey, Layout)>);
impl Before {
    pub(crate) fn layout(shell: &Shell, key: &LayoutKey) -> Self {
        Self(
            shell
                .areas
                .layout
                .book
                .borrow()
                .as_ref()
                .and_then(|book| book.get(key))
                .map(|layout| (key.clone(), layout.clone()))
                .into_iter()
                .collect(),
        )
    }
    pub(crate) fn book(shell: &Shell) -> Self {
        Self(
            shell
                .areas
                .layout
                .book
                .borrow()
                .as_ref()
                .map(|book| {
                    book.iter()
                        .map(|(key, layout)| (key.clone(), layout.clone()))
                        .collect()
                })
                .unwrap_or_default(),
        )
    }
    pub(crate) fn alias(shell: &Shell, name: &vessel_app_view::machine_book::Name) -> Self {
        let mut snapshot = Self::book(shell);
        snapshot.0.retain(|(key, _)| {
            matches!(key,
            LayoutKey::Session { machine, .. } if machine.as_str() == name.as_str())
        });
        snapshot
    }
    fn changes(self, shell: &Shell, source: Source) -> Vec<Notice> {
        let book = shell.areas.layout.book.borrow();
        let mut notices = Vec::new();
        for (key, before) in self.0 {
            for old in before.tabs() {
                let current = book
                    .as_ref()
                    .and_then(|book| book.get(&key))
                    .and_then(|layout| layout.tab(old.id()));
                let reason = match source {
                    Source::Unavailable => Reason::Unavailable,
                    Source::Local | Source::Canonical
                        if current.is_some_and(|tab| tab.target() == old.target()) =>
                    {
                        continue;
                    }
                    Source::Canonical => Reason::CanonicalReplacement,
                    Source::Local if current.is_some() => Reason::Retargeted,
                    Source::Local => Reason::Closed,
                };
                notices.push(Notice {
                    workspace: key.clone(),
                    tab: old.id(),
                    target: old.target().clone(),
                    reason,
                });
            }
        }
        notices
    }
}
impl Shell {
    pub(crate) fn layout_register_retirement(
        &mut self,
        owner: Owner,
        hook: Hook,
    ) -> Result<(), LayoutError> {
        self.areas.layout.retirements.register(owner, hook)
    }
    /// Gather before callbacks, release every book borrow, then synchronously
    /// invalidate owner admissions. Hooks must not await, issue RPC or reenter layout.
    pub(crate) fn layout_retired(
        &mut self,
        before: Before,
        source: Source,
        cx: &mut Context<'_, Self>,
    ) {
        for notice in before.changes(self, source) {
            let hook = self.areas.layout.retirements.hook(notice.target());
            if let Some(hook) = hook {
                hook(self, &notice, cx);
            }
        }
    }
}

#[cfg(test)]
mod tests;
