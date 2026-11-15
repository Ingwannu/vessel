//! Comparable draw inputs prepared at data/language/clock updates, not in Render.
use super::{Area, Popup, RowControl};
use vessel_app_kit::{select::Anchor, ui::SharedString};
use vessel_app_view::{
    Fleet,
    localization::Translator,
    schedules::{
        locale,
        presentation::{self, Notice},
    },
};
impl Area {
    pub(super) fn rebuild(&mut self, fleet: &Fleet, catalog: &Translator) -> bool {
        let fallback = vessel_app_view::schedules::clock::Clock::unknown();
        let clock = self.data.clock.as_ref().unwrap_or(&fallback);
        let rows = presentation::rows(&self.data.page, fleet, chrono::Utc::now(), clock, catalog);
        let body = self.data.page.body();
        let can_create = self.data.page.can_create();
        let host_errors: Vec<SharedString> = self
            .data
            .page
            .problems()
            .into_iter()
            .map(|(key, failure)| {
                catalog
                    .schedules()
                    .host_error(key.as_str(), locale::failure(failure, catalog))
                    .unwrap_or_else(|_| catalog.schedules().display_unavailable().to_owned())
                    .into()
            })
            .collect();
        let blocking = self
            .data
            .page
            .blocking_explanations()
            .into_iter()
            .map(|(key, failure)| Notice {
                label: catalog
                    .schedules()
                    .host_error(key.as_str(), locale::failure(failure, catalog))
                    .unwrap_or_else(|_| catalog.schedules().display_unavailable().to_owned())
                    .into(),
                variant: failure.variant(),
            })
            .collect::<Vec<_>>();
        let local = self.data.page.local_failure.as_ref().map(|failure| Notice {
            label: locale::failure(failure, catalog).into(),
            variant: failure.variant(),
        });
        let history = vessel_app_view::schedules::history::presentation::prepare(
            &self.data.history,
            clock,
            catalog,
        )
        .map_err(|error| Notice {
            label: error.label(catalog).into(),
            variant: vessel_app_view::alert::Variant::Info,
        });
        let editor = vessel_app_view::schedules::editor::presentation::prepare(
            &self.data.editor,
            fleet,
            clock,
            catalog,
        )
        .map_err(|error| Notice {
            label: error.label(catalog).into(),
            variant: vessel_app_view::alert::Variant::Info,
        });
        let changed = self.data.can_create != can_create
            || self.data.editor_cache != editor
            || self.data.history_cache != history
            || self.data.rows != rows
            || self.data.body != body
            || self.data.host_errors != host_errors
            || self.data.blocking != blocking
            || self.data.local != local;
        self.data.editor_cache = editor;
        self.data.history_cache = history;
        self.data.rows = rows;
        self.data.body = body;
        self.data.can_create = can_create;
        self.data.host_errors = host_errors;
        self.data.blocking = blocking;
        self.data.local = local;
        self.reconcile_controls();
        changed
    }
    fn reconcile_controls(&mut self) {
        let ui = &mut self.ui;
        if ui.selected.as_ref().is_some_and(|intent| {
            !intent.is_current(&self.data.page)
                || !self
                    .data
                    .rows
                    .iter()
                    .any(|row| row.key == *intent.key() && row.has_menu())
        }) {
            ui.selected = None;
            if ui.menu.is_open() {
                ui.menu.close();
                let _new = ui.pending_focus.insert(Popup::Actions);
            }
        }
        if ui
            .deleting
            .as_ref()
            .is_some_and(|prompt| !prompt.intent.is_current(&self.data.page))
        {
            ui.deleting = None;
            let _new = ui.pending_focus.insert(Popup::Delete);
        }
        if ui.picker.is_open()
            && (!self.data.page.filters_visible() || !self.data.page.multiple_machines())
        {
            ui.picker.close();
            let _new = ui.pending_focus.insert(Popup::Hosts);
        }
        ui.anchors
            .retain(|key, _| self.data.rows.iter().any(|row| row.key == *key));
        for row in &self.data.rows {
            let _existing = ui
                .anchors
                .entry(row.key.clone())
                .or_insert_with(|| RowControl {
                    anchor: Anchor::default(),
                    id: format!(
                        "schedule-actions-{}-{}",
                        row.key.machine.as_str(),
                        row.key.schedule
                    )
                    .into(),
                });
        }
    }
}
