//! Staged timeline owner adapters over the ONE layout-owned resource registry.
//! No production path, transport authority, raw client or selected-global fallback
//! is enabled here. W12 owns the eventual coordinated storage/follow cutover.

use super::{ConversationArea, origin::Origin};
use crate::shell::Shell;
use vessel_app_kit::ui::App;
use vessel_app_view::prefs::{ConversationDisplay, ReasoningExpansion};
use vessel_app_view::residents::{
    FollowBinding, HoldReason, OperationKind, OperationTicket, OwnerReadiness, PanelFocus,
    ResidentProblem, ResidentToken, SessionContext, Visibility,
};
use vessel_proto::EventRecord;

mod seat;
pub(crate) use seat::ResidentSeat;

/// Moves the concrete owner resource, preserving scroll/controller identity.
/// This is not serialization or a claim that a cold transcript has a byte cap.
pub(crate) struct TimelineSnapshot {
    source: ResidentToken,
    area: Box<ConversationArea>,
}
impl TimelineSnapshot {
    /// W12 first admits the destination. A new incarnation of the SAME resource
    /// may reuse local data; another window/node/session must retain the snapshot.
    /// This comparison never admits a token or refreshes network authority.
    pub(crate) fn restore(self, destination: ResidentToken) -> Result<ConversationArea, Box<Self>> {
        if self.source.registry != destination.registry
            || self.source.address != destination.address
        {
            return Err(Box::new(self));
        }
        Ok(*self.area)
    }
}
impl std::fmt::Debug for TimelineSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimelineSnapshot")
            .field("entries", &self.area.timeline.entries().len())
            .finish_non_exhaustive()
    }
}

impl ConversationArea {
    /// Base-compatible empty constructor; the admitted owner supplies identity.
    /// Full A7 adds reasoning/turn presentation through the wrapper below.
    #[must_use]
    pub(crate) fn with_display(display: ConversationDisplay, cx: &mut App) -> Self {
        let mut area = Self::new(cx);
        area.collapse = super::area::collapsing(Some(display));
        area.folds = area
            .collapse
            .then(vessel_app_view::timeline::Folds::default);
        area
    }

    /// A new admitted resource uses the kept Chat choices, not whichever empty
    /// defaults happened to seed the old selected window. Identity/admission
    /// remains entirely in the registry; this constructor launches no follow.
    pub(crate) fn with_presentation(
        display: ConversationDisplay,
        reasoning: ReasoningExpansion,
        cx: &mut App,
    ) -> Self {
        let mut area = Self::with_display(display, cx);
        area.turns = super::turns::enabled(Some(display)).then(super::turns::Turns::default);
        area.folds = area
            .collapse
            .then(vessel_app_view::timeline::Folds::default);
        area.reasoning = reasoning;
        area
    }

    /// W1 supplies its ONE tree-aggregated navigation hold; A8/lifecycle vote in
    /// the same registry. Origin recursion below checks local protection only,
    /// never a fabricated default navigation Ready. Forms stay with their owner.
    pub(crate) fn snapshot_readiness(&self, navigation: OwnerReadiness) -> OwnerReadiness {
        match navigation {
            OwnerReadiness::Hold(_) => navigation,
            OwnerReadiness::Ready => self.local_snapshot_readiness(),
        }
    }

    fn local_snapshot_readiness(&self) -> OwnerReadiness {
        if self.find.is_some() || self.rename.is_some() || self.queue_edit.is_some() {
            return OwnerReadiness::Hold(HoldReason::ActiveEditor);
        }
        if self.tool_sheet.snapshot_busy()
            || self.confirm.is_some()
            || self.menu.is_open()
            || self.forking.menu.is_open()
            || self.rewinding.menu.is_open()
        {
            return OwnerReadiness::Hold(HoldReason::Overlay);
        }
        if self.tail.snapshot_busy()
            || self.read.snapshot_busy()
            || self.forking.snapshot_busy()
            || self.rewinding.snapshot_busy()
            || self.restarting
        {
            return OwnerReadiness::Hold(HoldReason::PendingEffect);
        }
        match &self.origin {
            Origin::Wanted { .. } | Origin::Reading { .. } => {
                OwnerReadiness::Hold(HoldReason::Loading)
            }
            Origin::Shown(area) => area.local_snapshot_readiness(),
            Origin::None | Origin::Unread { .. } => OwnerReadiness::Ready,
        }
    }

    /// W12 must collect every owner's Ready before retiring the epoch. A refusal
    /// returns the exact live resource rather than defaulting or copying it.
    ///
    /// # Errors
    /// Protected timeline/navigation state cannot be destructively transferred.
    pub(crate) fn into_timeline_snapshot(
        self,
        source: ResidentToken,
        navigation: OwnerReadiness,
    ) -> Result<TimelineSnapshot, Box<Self>> {
        if self.snapshot_readiness(navigation) != OwnerReadiness::Ready
            || self
                .shown
                .as_ref()
                .is_some_and(|shown| shown.session != source.address.session)
        {
            return Err(Box::new(self));
        }
        Ok(TimelineSnapshot {
            source,
            area: Box::new(self),
        })
    }
}

impl Shell {
    /// A view borrows only its exact current presentation; missing never means selected.
    pub(crate) fn conversation_scoped(
        &self,
        context: &SessionContext,
    ) -> Result<&ConversationArea, ResidentProblem> {
        Ok(&self
            .areas
            .session_residents
            .resources(context)?
            .conversation)
    }

    /// Local input additionally requires the current visible/focused presentation.
    /// Field owners may refresh the SAME identity/claim before this call; network
    /// replies never use that refresh path.
    pub(crate) fn conversation_input_scoped(
        &mut self,
        context: &SessionContext,
    ) -> Result<&mut ConversationArea, ResidentProblem> {
        let _current = self.areas.session_residents.resources(context)?;
        if context.visibility != Visibility::Visible || context.focus != PanelFocus::Focused {
            return Err(ResidentProblem::Held);
        }
        Ok(&mut self
            .areas
            .session_residents
            .resources_mut(context)?
            .conversation)
    }

    /// The link must first validate G4's original delivery lease after await.
    /// Fresh hidden records are accepted through the original follow binding.
    pub(crate) fn conversation_records_scoped(
        &mut self,
        binding: FollowBinding,
        records: &[EventRecord],
    ) -> Result<bool, ResidentProblem> {
        self.areas.session_residents.receive(binding, |resources| {
            resources.conversation.fold_records(records)
        })
    }

    /// Check concrete owner correlation before consuming its canonical ticket.
    /// This stays in one synchronous UI update; original lease admission is the
    /// caller's prerequisite, and hidden ingress never refreshes selected state.
    pub(crate) fn conversation_settle_if_scoped<T>(
        &mut self,
        binding: FollowBinding,
        ticket: OperationTicket,
        validate: impl FnOnce(&ConversationArea) -> bool,
        apply: impl FnOnce(&mut ConversationArea) -> T,
    ) -> Result<Option<T>, ResidentProblem> {
        let matches = self
            .areas
            .session_residents
            .receive(binding, |resources| validate(&resources.conversation))?;
        if !matches {
            return Ok(None);
        }
        self.conversation_settled_scoped(binding, ticket, apply)
            .map(Some)
    }

    /// Consume a concrete owner's admitted effect once BEFORE changing input/state.
    pub(crate) fn conversation_settled_scoped<T>(
        &mut self,
        binding: FollowBinding,
        ticket: OperationTicket,
        apply: impl FnOnce(&mut ConversationArea) -> T,
    ) -> Result<T, ResidentProblem> {
        self.areas
            .session_residents
            .settle(binding, OperationKind::Timeline, ticket, |resources| {
                apply(&mut resources.conversation)
            })
    }
}

#[cfg(test)]
mod tests;
