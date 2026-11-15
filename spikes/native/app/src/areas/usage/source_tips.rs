//! Existing observation tickets retire tooltip places with their report set.
use super::rows::Density;
use std::{cell::RefCell, collections::BTreeMap};
use vessel_app_kit::{
    Button, Tokens, Tooltips,
    tooltip::Place,
    ui::{
        AnyElement, Div, InteractiveElement as _, ParentElement as _, SharedString, Stateful, div,
    },
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
        refresh: Button,
        entry: &SourceReportEntry,
        freshness: &str,
        density: Density,
    ) -> Stateful<Div> {
        let mut state = self.0.borrow_mut();
        let place = state.places.entry(entry.id().clone()).or_default().clone();
        let refresh = div()
            .id(SharedString::from(format!(
                "usage-report-freshness-{}",
                entry.id()
            )))
            .child(refresh);
        state.tips.on(
            refresh,
            &place,
            freshness.to_owned(),
            matches!(density, Density::Compact),
        )
    }
    pub(super) fn render(&self, tokens: &Tokens) -> Option<AnyElement> {
        self.0.borrow().tips.render(tokens)
    }
}
