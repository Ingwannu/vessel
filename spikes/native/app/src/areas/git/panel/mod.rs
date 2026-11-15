//! Feature-owned C3 pane state. Layout supplies identity; G4 supplies admission.
//!
//! [Decision Log]
//! - Reuse the sole pane registry, typed panel tickets and selected connection.
//! - Root discovery names only the exact legacy Session layout key. A real
//!   Workspace key waits for its authoritative registry adapter, with no cwd,
//!   selected-session or first-alias fallback.
//! - Each answer must still match its original owner, target and admission.
//!   Lifetime keys belong to `GitArea`, never a disposable pane entry.
//! - This registration is the package skeleton; document/editor mounting is
//!   added on the same state rather than a second panel or transport.
mod comparison;
mod draw;
mod lifecycle;

use std::collections::HashMap;
use vessel_app_kit::VirtualList;
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::workspace::git::panel::{
    Comparison, ComparisonMode, PanelAddress, PanelBinding, PanelReads, PanelTarget, ReadFailure,
    Whitespace,
};

#[derive(Default)]
pub(super) struct Panels {
    entries: HashMap<PanelAddress, Entry>,
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
}
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
