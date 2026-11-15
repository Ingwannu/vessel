//! Route and action wiring. The timer updates cached words only, never polls
//! schedule/list; reads follow arrival, reconnect and explicit invalidation.
use super::DeletePrompt;
use crate::{routes::Route, shell::Shell};
use std::time::Duration;
use vessel_app_kit::{
    select::Anchor,
    ui::{Context, Window},
};
use vessel_app_link::{Command, schedules::Message};
use vessel_app_view::schedules::{Action, load::MachineKey, page::RowKey};
const METADATA_EVERY: Duration = Duration::from_secs(60);

pub(crate) fn arrive(shell: &mut Shell, _window: &mut Window, cx: &mut Context<'_, Shell>) {
    shell.schedules_fleet();
    if !shell.areas.schedules.ticking {
        shell.areas.schedules.ticking = true;
        tick(cx);
    }
}
fn tick(cx: &mut Context<'_, Shell>) {
    cx.spawn(async move |this, cx| {
        loop {
            cx.background_executor().timer(METADATA_EVERY).await;
            let alive = this.update(cx, |shell, cx| {
                if shell.route != Route::Schedules {
                    shell.areas.schedules.ticking = false;
                    return false;
                }
                if shell.schedules_rebuild() {
                    cx.notify();
                }
                true
            });
            if !matches!(alive, Ok(true)) {
                break;
            }
        }
    })
    .detach();
}
impl Shell {
    pub(crate) fn schedules_fleet(&mut self) {
        if self.loaded {
            let keys: Vec<_> = self
                .fleet
                .machines
                .iter()
                .filter_map(|machine| MachineKey::of(&machine.name))
                .collect();
            self.areas.schedules.data.page.machines(&keys);
        }
        if self.route == Route::Schedules {
            self.schedules_due();
            let _changed = self.schedules_rebuild();
        }
    }
    pub(crate) fn schedules_due(&mut self) {
        if self.route == Route::Schedules {
            for request in self.areas.schedules.data.page.due(chrono::Utc::now()) {
                self.command(Command::Schedules(request));
            }
        }
    }
    pub(crate) fn schedules_message(&mut self, message: Message) -> bool {
        self.areas.schedules.data.message(message);
        self.schedules_due();
        self.route == Route::Schedules && self.schedules_rebuild()
    }
    pub(crate) fn schedules_rebuild(&mut self) -> bool {
        self.areas.schedules.rebuild(&self.fleet, self.words)
    }
    pub(super) fn schedule_menu(
        &mut self,
        key: &RowKey,
        anchor: &Anchor,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self
            .areas
            .schedules
            .data
            .rows
            .iter()
            .any(|row| row.key == *key && !row.menu.is_empty())
        {
            return;
        }
        let Some(bounds) = anchor.get() else {
            return;
        };
        let ui = &mut self.areas.schedules.ui;
        ui.selected = Some(key.clone());
        ui.menu.open_end(bounds, window, cx);
        cx.notify();
    }
    pub(super) fn schedule_act(&mut self, key: &RowKey, action: Action) {
        if let Some(request) = self.areas.schedules.data.page.act(key, action) {
            self.command(Command::Schedules(request));
            let _changed = self.schedules_rebuild();
        }
    }
    pub(super) fn schedule_delete_prompt(
        &mut self,
        key: &RowKey,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(row) = self.areas.schedules.data.page.row(key) else {
            return;
        };
        if !row.actions().contains(&Action::Delete) {
            return;
        }
        let title = self.t.lang.schedule_action(
            vessel_app_view::schedules::product(&row.schedule.definition.target),
            Action::Delete,
        );
        let body = self
            .words
            .schedules()
            .delete_confirm(vessel_app_view::schedules::title(&row.schedule.definition))
            .unwrap_or_else(|_| self.words.schedules().display_unavailable().to_owned());
        let ui = &mut self.areas.schedules.ui;
        ui.deleting = Some(DeletePrompt {
            key: key.clone(),
            title: title.into(),
            body: body.into(),
        });
        ui.dialog.focus(window, cx);
    }
}

pub(crate) fn sidebar(
    shell: &Shell,
    tokens: &vessel_app_kit::Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> Option<vessel_app_kit::ui::AnyElement> {
    use vessel_app_kit::ui::{IntoElement as _, StatefulInteractiveElement as _};
    Some(
        crate::areas::sidebar::nav_row(
            "nav-schedules",
            shell.t.lang.schedules().title,
            Some(vessel_app_kit::assets::Glyph::CalendarClock),
            shell.route == Route::Schedules,
            touch,
            tokens,
        )
        .on_click(cx.listener(|shell, _, window, cx| shell.go(Route::Schedules, window, cx)))
        .into_any_element(),
    )
}
pub(crate) fn leaving(
    route: Route,
    shell: &mut Shell,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    if route != Route::Schedules {
        let ui = &mut shell.areas.schedules.ui;
        ui.menu.close_and_restore(window, cx);
        ui.picker.close();
        ui.deleting = None;
    }
}
