//! One default/explicit-choice rule serves tools and reasoning on every seat.

use vessel_app_view::prefs::ReasoningExpansion;
use vessel_app_view::timeline::{EntryKind, Folds, PlanExpansion, plan_outcome, plan_text};

use super::ConversationArea;
use crate::shell::Shell;

impl ConversationArea {
    /// Plans default by outcome; tool status and reasoning preferences are defaults
    /// only. An explicit choice survives later completion and preference changes.
    pub(super) fn entry_expanded(&self, index: usize) -> bool {
        if let Some(tool) = self
            .timeline
            .entries()
            .get(index)
            .and_then(|entry| entry.kind.as_tool())
            .filter(|tool| plan_text(tool).is_some())
        {
            let outcome = plan_outcome(self.timeline.entries(), &tool.id);
            return self.plans.expansion(&tool.id, outcome) == PlanExpansion::Expanded;
        }
        self.disclosures.expanded(index, self.entry_default(index))
    }

    pub(super) fn entry_default(&self, index: usize) -> bool {
        match self.timeline.entries().get(index).map(|entry| &entry.kind) {
            Some(EntryKind::Thought { .. }) => self.reasoning.expanded(),
            Some(EntryKind::Tool(tool)) => {
                tool.working() && !self.timeline.asks_permission(&tool.id)
            }
            Some(
                EntryKind::Ready { .. }
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
        self.reasoning = expansion;
        self.folds = self
            .collapse
            .then(|| Folds::with_reasoning(self.timeline.entries(), expansion.expanded()));
        self.open_runs.clear();
        self.rows_again();
        self.list.reset(self.at(self.rows.len()));
        self.tail_dropped();
    }
}

impl Shell {
    /// Apply the kept Chat default to the session and its displayed origin.
    pub(crate) fn conversation_reasoning(&mut self, expansion: ReasoningExpansion) {
        if self.areas.conversation.reasoning == expansion {
            return;
        }
        self.areas.conversation.set_reasoning(expansion);
        if let super::origin::Origin::Shown(origin) = &mut self.areas.conversation.origin {
            origin.set_reasoning(expansion);
        }
        let area = &self.areas.conversation;
        area.list.reset(area.at(area.rows.len()));
        area.tail_dropped();
    }
}
