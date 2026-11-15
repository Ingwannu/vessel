//! One default/explicit-choice rule serves tools and reasoning on every seat.

use vessel_app_view::prefs::ReasoningExpansion;
use vessel_app_view::timeline::{EntryKind, Folds, PlanExpansion, plan_outcome, plan_text};

use super::ConversationArea;
use crate::shell::Shell;

impl ConversationArea {
    /// Opening a lone call pre-opens the group it will host when another arrives
    /// (`tool-calls/expansion.ts`). Automatic running defaults never do this.
    pub(super) fn remember_standalone(&mut self, index: usize) {
        if !self
            .timeline
            .entries()
            .get(index)
            .is_some_and(|entry| entry.kind.folds())
        {
            return;
        }
        let grouped = self
            .folds
            .as_ref()
            .and_then(|folds| folds.run_of(index))
            .is_some();
        let detailed_turn = self.turns.as_ref().is_some_and(|turns| {
            turns
                .folds
                .containing(index)
                .is_some_and(|turn| turns.open.contains(&turn.host))
        });
        if grouped && !detailed_turn {
            return;
        }
        if self.entry_expanded(index) {
            let _new = self.open_runs.insert(index);
        } else {
            let _old = self.open_runs.remove(&index);
        }
    }

    /// Plans default by outcome; ordinary tools start closed, and reasoning uses
    /// its preference. Explicit choices survive completion and preference changes.
    pub(super) fn entry_expanded(&self, index: usize) -> bool {
        if let Some(tool) = self
            .timeline
            .entries()
            .get(index)
            .and_then(|entry| entry.kind.as_tool())
            .filter(|tool| plan_text(*tool).is_some())
        {
            let outcome = plan_outcome(self.timeline.entries(), &tool.id);
            return self.plans.expansion(&tool.id, outcome) == PlanExpansion::Expanded;
        }
        self.disclosures.expanded(index, self.entry_default(index))
    }

    pub(super) fn entry_default(&self, index: usize) -> bool {
        match self.timeline.entries().get(index).map(|entry| &entry.kind) {
            Some(EntryKind::Thought { .. }) => self.reasoning.expanded(),
            Some(
                EntryKind::Ready { .. }
                | EntryKind::Tool(_)
                | EntryKind::Prompt(_)
                | EntryKind::Steer { .. }
                | EntryKind::Question { .. }
                | EntryKind::Message { .. }
                | EntryKind::Plan { .. }
                | EntryKind::Permission(_)
                | EntryKind::TurnEnded { .. }
                | EntryKind::TurnFailed { .. }
                | EntryKind::Log { .. }
                | EntryKind::Status { .. }
                | EntryKind::Parking(_)
                | EntryKind::Forked { .. }
                | EntryKind::Profile { .. }
                | EntryKind::Notification(_)
                | EntryKind::Compaction(_)
                | EntryKind::Plugin(_)
                | EntryKind::Model(_),
            )
            | None => false,
        }
    }

    /// Embedded conversations use this same preference boundary before feeding
    /// records; no second preference store is needed.
    pub(crate) fn set_reasoning(&mut self, expansion: ReasoningExpansion) {
        if self.reasoning == expansion {
            return;
        }
        let bookmark = self.reading_bookmark();
        self.reasoning = expansion;
        self.folds = self
            .collapse
            .then(|| Folds::with_reasoning(self.timeline.entries(), expansion.expanded()));
        self.rows_again();
        self.list.reset(self.at(self.rows.len()));
        self.tail_dropped();
        self.restore_reading(bookmark);
    }
}

impl Shell {
    /// Apply the kept Chat default to the session and its displayed origin.
    pub(crate) fn conversation_reasoning(&mut self, expansion: ReasoningExpansion) {
        if self.areas.conversation.reasoning == expansion {
            return;
        }
        let bookmark = self.areas.conversation.reading_bookmark();
        self.areas.conversation.set_reasoning(expansion);
        if let super::origin::Origin::Shown(origin) = &mut self.areas.conversation.origin {
            origin.set_reasoning(expansion);
        }
        let area = &self.areas.conversation;
        area.list.reset(area.at(area.rows.len()));
        area.tail_dropped();
        area.restore_reading(bookmark);
    }
}
