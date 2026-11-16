//! Existing observation tickets retire tooltip places with their report set.
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use vessel_app_kit::{
    Tokens, Tooltips, toolbar_button::ToolbarButton, tooltip::Place, ui::AnyElement,
};
use vessel_app_view::devices::Ticket;
use vessel_proto::usage_reports::{SourceReportEntry, UsageReportId, UsageReports};
#[derive(Debug, Default)]
pub(super) struct SourceTips(RefCell<Observed>);
#[derive(Debug, Default)]
struct Observed {
    ticket: Option<Ticket>,
    tips: Tooltips,
    places: BTreeMap<UsageReportId, Place>,
    actions: BTreeMap<&'static str, Place>,
    /// Saved sign-in cards, keyed by their stable control id (harness/profile).
    saved: BTreeMap<String, Place>,
}
impl SourceTips {
    pub(super) fn prepare(&self, ticket: Option<Ticket>, reports: &UsageReports) {
        let mut state = self.0.borrow_mut();
        if state.ticket != ticket {
            *state = Observed {
                ticket,
                ..Observed::default()
            };
        }
        state.places.retain(|id, _| {
            reports
                .reports
                .as_slice()
                .iter()
                .any(|entry| entry.id() == id)
        });
    }
    pub(super) fn on(
        &self,
        refresh: ToolbarButton,
        entry: &SourceReportEntry,
        freshness: &str,
    ) -> ToolbarButton {
        let mut state = self.0.borrow_mut();
        let place = Rc::clone(state.places.entry(entry.id().clone()).or_default());
        refresh.tooltip_text(
            &state.tips,
            &place,
            freshness.to_owned(),
            vessel_app_kit::tooltip::Side::Top,
        )
    }
    /// The saved sign-in card's Refresh tooltip says when its values were read.
    pub(super) fn on_saved(
        &self,
        refresh: ToolbarButton,
        key: &str,
        freshness: &str,
    ) -> ToolbarButton {
        let mut state = self.0.borrow_mut();
        let place = Rc::clone(state.saved.entry(key.to_owned()).or_default());
        refresh.tooltip_text(
            &state.tips,
            &place,
            freshness.to_owned(),
            vessel_app_kit::tooltip::Side::Top,
        )
    }
    pub(super) fn action(&self, button: ToolbarButton, id: &'static str) -> ToolbarButton {
        let mut state = self.0.borrow_mut();
        let place = Rc::clone(state.actions.entry(id).or_default());
        button.tooltip(&state.tips, &place)
    }
    pub(super) fn render(&self, tokens: &Tokens) -> Option<AnyElement> {
        self.0.borrow().tips.render(tokens)
    }
}
