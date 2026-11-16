//! Feature-owned C3 pane state. Layout supplies identity; G4 supplies admission.
//!
//! [Decision Log]
//! - Reuse the sole pane registry, typed panel tickets and selected connection.
//! - Root discovery names only the exact legacy Session layout key. A real
//!   Workspace key waits for its authoritative registry adapter, with no cwd,
//!   selected-session or first-alias fallback.
//! - Each answer must still match its original owner, target and admission.
//!   Lifetime keys belong to `GitArea`, never a disposable pane entry.
//! - Lazy unified bodies extend this same entry/read store. Document resources
//!   are bounded globally; no second panel, ticket book or transport is added.
mod bodies;
mod comparison;
mod document;
mod draw;
mod file;
mod lifecycle;

use std::collections::{HashMap, VecDeque};
use vessel_app_kit::VirtualList;
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::workspace::git::panel::{
    Comparison, ComparisonMode, PanelAddress, PanelBinding, PanelReads, PanelTarget, ReadFailure,
    Whitespace,
};

#[derive(Default)]
pub(super) struct Panels {
    entries: HashMap<PanelAddress, Entry>,
    registration: super::retirement::Registration,
    body_order: VecDeque<bodies::BodyAddress>,
}
struct Entry {
    target: PanelTarget,
    // The discovery key remains the owner incarnation after its reply settles.
    // Same-root Refresh must not revive a callback from the previous entry.
    instance: Option<vessel_app_view::workspace::git::RequestKey>,
    comparison: Comparison,
    reason: vessel_proto::workspace::GitReadReason,
    context: Option<ConnectionContext>,
    binding: PanelBinding,
    reads: Option<PanelReads>,
    problem: Option<ReadFailure>,
    files: VirtualList,
    tips: vessel_app_kit::Tooltips,
    path_places: HashMap<vessel_proto::workspace::RelPath, vessel_app_kit::tooltip::Place>,
    documents: HashMap<vessel_proto::workspace::RelPath, document::Document>,
}
impl Entry {
    fn new(target: PanelTarget, context: Option<ConnectionContext>) -> Self {
        let comparison = match &target {
            PanelTarget::CommitDiff(id) => Comparison::at_commit(id.clone(), Whitespace::Show),
            PanelTarget::Changes | PanelTarget::WorkingDiff => {
                Comparison::new(ComparisonMode::All, None, Whitespace::Show)
            }
        };
        Self {
            target,
            instance: None,
            comparison,
            reason: vessel_proto::workspace::GitReadReason::View,
            context,
            binding: PanelBinding::default(),
            reads: None,
            problem: None,
            files: VirtualList::new(false),
            tips: vessel_app_kit::Tooltips::new(),
            path_places: HashMap::new(),
            documents: HashMap::new(),
        }
    }
    // A manual refresh rediscovers authority, but must not reset the person's
    // chosen comparison. Retain only presentation from the same target/context;
    // old read tickets, roots, failures and future editor state are never copied.
    fn retain_comparison(&mut self, previous: &Self) {
        if self.target == previous.target && self.context == previous.context {
            self.comparison = previous
                .reads
                .as_ref()
                .map_or(&previous.comparison, |reads| reads.comparison())
                .clone();
        }
    }

    fn matches_admission(
        &self,
        instance: vessel_app_view::workspace::git::RequestKey,
        target: &PanelTarget,
        context: &ConnectionContext,
        current: Option<&ConnectionContext>,
    ) -> bool {
        self.instance == Some(instance)
            && &self.target == target
            && self.context.as_ref() == Some(context)
            && current == Some(context)
    }
}

impl Panels {
    /// Other Git entry points share the same startup refusal as the pane.
    pub(super) fn owner_problem(&self) -> Option<ReadFailure> {
        self.registration.problem()
    }

    /// No pre-initialization UI may acquire admission just because startup ran.
    pub(super) fn initialize(&mut self, result: Result<(), vessel_app_view::layout::LayoutError>) {
        // Pre-start/test data cannot be promoted into an initialized owner.
        // These entries own no sent task or mutation receipt.
        self.entries.clear();
        self.body_order.clear();
        self.registration = match result {
            Ok(()) => super::retirement::Registration::Ready,
            Err(why) => super::retirement::Registration::Refused(why),
        };
    }

    /// Match the original notice so equal tab ids in other workspaces survive.
    pub(super) fn retire_owner(
        &mut self,
        notice: &crate::areas::layout::registry::retirement::Notice,
    ) -> bool {
        let address = PanelAddress {
            workspace: notice.workspace().clone(),
            tab: notice.tab(),
        };
        if self
            .entries
            .get(&address)
            .is_some_and(|entry| layout_target(&entry.target) == *notice.target())
        {
            let _removed = self.entries.remove(&address);
            true
        } else {
            false
        }
    }
}
pub(super) use draw::owner_notice;
pub(crate) use draw::render_panel;

#[cfg(test)]
mod settlement_tests;

// Adapt feature identity to the ONE layout query; never keep a second target book.
#[must_use]
fn layout_target(target: &PanelTarget) -> vessel_app_view::layout::Target {
    match target {
        PanelTarget::Changes => vessel_app_view::layout::Target::Changes,
        PanelTarget::WorkingDiff => vessel_app_view::layout::Target::WorkingDiff,
        PanelTarget::CommitDiff(id) => vessel_app_view::layout::Target::CommitDiff(id.clone()),
    }
}

#[cfg(test)]
mod target_tests;

#[cfg(test)]
mod owner_tests;
