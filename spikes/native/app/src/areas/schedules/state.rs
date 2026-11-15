//! Comparable draw inputs prepared at data/language/clock updates, not in Render.
use super::{Area, RowControl};
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
        let rows = presentation::rows(
            &self.data.page,
            fleet,
            chrono::Utc::now(),
            &self.data.clock,
            catalog,
        );
        let body = self.data.page.body();
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
        let changed = self.data.rows != rows
            || self.data.body != body
            || self.data.host_errors != host_errors
            || self.data.blocking != blocking
            || self.data.local != local;
        self.data.rows = rows;
        self.data.body = body;
        self.data.host_errors = host_errors;
        self.data.blocking = blocking;
        self.data.local = local;
        {
            let ui = &mut self.ui;
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
        changed
    }
}
