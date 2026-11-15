//! Route and action wiring. The timer updates cached words only, never polls
//! schedule/list; reads follow arrival, reconnect and explicit invalidation.
use super::{DeletePrompt, Tick};
use crate::{routes::Route, shell::Shell};
use vessel_app_kit::{
    select::Anchor,
    ui::{Context, Window},
};
use vessel_app_link::{Command, schedules::Message};
use vessel_app_view::schedules::{Action, load::MachineKey, page::RowKey};
pub(crate) fn arrive(shell: &mut Shell, _window: &mut Window, cx: &mut Context<'_, Shell>) {
    shell.schedules_fleet(cx);
    shell.schedules_due();
    let _changed = shell.schedules_changed(cx);
}
impl Shell {
    pub(crate) fn schedules_fleet(&mut self, cx: &mut Context<'_, Self>) {
        if self.loaded {
            let mut keys: Vec<_> = self
                .fleet
                .machines
                .iter()
                .filter_map(|machine| MachineKey::of(&machine.name))
                .collect();
            keys.sort();
            let changed = !self.areas.schedules.data.page.keys().eq(keys.iter());
            self.areas.schedules.data.page.machines(&keys);
            if changed {
                self.schedules_due();
            }
        }
        if self.route == Route::Schedules {
            let _changed = self.schedules_changed(cx);
        }
    }
    pub(crate) fn schedules_due(&mut self) {
        if self.route == Route::Schedules {
            for request in self.areas.schedules.data.page.due(chrono::Utc::now()) {
                self.command(Command::Schedules(request));
            }
        }
    }
    pub(crate) fn schedules_message(
        &mut self,
        message: Message,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        self.areas.schedules.data.message(message);
        self.schedules_due();
        self.route == Route::Schedules && self.schedules_changed(cx)
    }
    pub(crate) fn schedules_changed(&mut self, cx: &mut Context<'_, Self>) -> bool {
        let changed = self.areas.schedules.rebuild(&self.fleet, self.words);
        self.schedules_clock(cx);
        changed
    }
    fn schedules_clock(&mut self, cx: &mut Context<'_, Self>) {
        let now = chrono::Utc::now();
        let wanted = (self.route == Route::Schedules)
            .then(|| {
                vessel_app_view::schedules::presentation::refresh_tier(
                    &self.areas.schedules.data.page,
                    now,
                )
            })
            .flatten();
        if self.areas.schedules.tick.as_ref().map(|tick| tick.tier) == wanted {
            return;
        }
        if let Some(old) = self.areas.schedules.tick.take() {
            old.cancel();
        }
        let Some(tier) = wanted else {
            return;
        };
        let task = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(tier.after(now)).await;
            let _closed = this.update(cx, |shell, cx| {
                let _finished = shell.areas.schedules.tick.take();
                if shell.schedules_changed(cx) {
                    cx.notify();
                }
            });
        });
        self.areas.schedules.tick = Some(Tick { tier, task });
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
    pub(super) fn schedule_act(
        &mut self,
        key: &RowKey,
        action: Action,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(request) = self.areas.schedules.data.page.act(key, action) {
            self.command(Command::Schedules(request));
            let _changed = self.schedules_changed(cx);
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
        if let Some(tick) = shell.areas.schedules.tick.take() {
            tick.cancel();
        }
        let ui = &mut shell.areas.schedules.ui;
        ui.menu.close_and_restore(window, cx);
        ui.picker.close();
        ui.deleting = None;
    }
}
