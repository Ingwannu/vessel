//! Route lifecycle, list invalidation and original-row actions. The clock task
//! changes relative text only; it does not poll the node.
use super::{DeletePrompt, Popup};
use crate::{routes::Route, shell::Shell};
use std::rc::Rc;
use vessel_app_kit::{
    Tokens,
    assets::Glyph,
    select::Anchor,
    ui::{AnyElement, Context, IntoElement as _, StatefulInteractiveElement as _, Window},
};
use vessel_app_link::{Command, schedules::Answer};
use vessel_app_view::{
    connection::ConnectionTag,
    schedules::{
        Action, is_heartbeat, load::MachineKey, locale::action_label, page::Intent, title,
    },
};

pub(crate) fn arrive(shell: &mut Shell, cx: &mut Context<'_, Shell>) {
    shell.areas.schedules.data.page.revisit(chrono::Utc::now());
    shell.schedules_fleet(cx);
    shell.schedules_due();
    let _changed = shell.schedules_changed(cx);
}

pub(crate) fn leaving(
    route: Route,
    shell: &mut Shell,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    if route == Route::Schedules {
        return;
    }
    if let Some(task) = shell.areas.schedules.tick.take() {
        drop(task);
    }
    shell.areas.schedules.editor.close(window, cx);
    let ui = &mut shell.areas.schedules.ui;
    ui.menu.close_and_restore(window, cx);
    ui.picker.close_and_restore(window, cx);
    ui.selected = None;
    if ui.deleting.take().is_some() || ui.pending_focus.contains(&Popup::Delete) {
        ui.dialog.dismissed(window, cx);
    }
    ui.pending_focus.clear();
}

pub(crate) fn sidebar(
    shell: &Shell,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    crate::areas::sidebar::nav_row(
        "nav-schedules",
        shell.words.sidebar().schedules(),
        Some(Glyph::CalendarClock),
        shell.route == Route::Schedules,
        touch,
        tokens,
    )
    .on_click(cx.listener(|shell, _, window, cx| shell.go(Route::Schedules, window, cx)))
    .into_any_element()
}

impl Shell {
    pub(crate) fn schedules_fleet(&mut self, cx: &mut Context<'_, Self>) {
        if self.loaded {
            let keys: Vec<_> = self
                .fleet
                .machines
                .iter()
                .filter_map(|machine| MachineKey::of(&machine.name))
                .collect();
            self.areas.schedules.data.page.machines(&keys);
            if self.areas.schedules.data.page.admissions(&self.fleet) {
                self.schedules_due();
            }
        }
        let _changed = self.schedules_changed(cx);
    }

    pub(crate) fn schedules_retired(
        &mut self,
        context: &ConnectionTag,
        cx: &mut Context<'_, Self>,
    ) {
        let retired = self.areas.schedules.editor.retired(context);
        let page_changed =
            self.areas.schedules.data.page.retired(context) && self.schedules_changed(cx);
        if retired || page_changed {
            cx.notify();
        }
    }

    pub(crate) fn schedules_due(&mut self) {
        if self.route == Route::Schedules {
            for request in self.areas.schedules.data.page.due() {
                self.command(Command::Schedules(request));
            }
        }
    }

    pub(crate) fn schedules_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        self.areas.schedules.data.answered(answer);
        self.schedules_due();
        if self.schedules_changed(cx) {
            cx.notify();
        }
    }

    pub(crate) fn schedules_changed(&mut self, cx: &mut Context<'_, Self>) -> bool {
        if self.route != Route::Schedules {
            return false;
        }
        let changed = self.areas.schedules.rebuild(&self.fleet, self.words);
        self.schedules_clock(cx);
        changed
    }

    fn schedules_clock(&mut self, cx: &mut Context<'_, Self>) {
        let now = chrono::Utc::now();
        let wanted = vessel_app_view::schedules::presentation::refresh_tier(
            &self.areas.schedules.data.page,
            now,
        );
        let tick = &mut self.areas.schedules.tick;
        if tick.as_ref().map(|(tier, _)| *tier) == wanted {
            return;
        }
        if let Some(old) = tick.take() {
            drop(old);
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
        self.areas.schedules.tick = Some((tier, task));
    }

    // A link reply has no Window. Restore any displaced focus on the next
    // frame with the same kit handlers used on a direct dismissal.
    pub(super) fn schedules_overlay_frame(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let ui = &mut self.areas.schedules.ui;
        for popup in std::mem::take(&mut ui.pending_focus) {
            match popup {
                Popup::Actions => ui.menu.close_and_restore(window, cx),
                Popup::Hosts => ui.picker.close_and_restore(window, cx),
                Popup::Delete => ui.dialog.dismissed(window, cx),
            }
        }
    }

    pub(super) fn schedule_menu(
        &mut self,
        intent: &Intent,
        anchor: &Anchor,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.schedules;
        if !intent.is_current(&area.data.page)
            || !area
                .data
                .rows
                .iter()
                .any(|row| row.key == *intent.key() && row.has_menu())
        {
            return;
        }
        let Some(bounds) = anchor.get() else {
            return;
        };
        let ui = &mut area.ui;
        let _removed = ui.pending_focus.remove(&Popup::Actions);
        ui.picker.close_and_restore(window, cx);
        ui.selected = Some(Rc::new(intent.clone()));
        ui.menu.open_end(bounds, window, cx);
        cx.notify();
    }

    pub(super) fn schedule_act(
        &mut self,
        intent: &Intent,
        action: Action,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(request) = self.areas.schedules.data.page.act_intent(intent, action) {
            self.command(Command::Schedules(request));
        }
        let _changed = self.schedules_changed(cx);
    }

    pub(super) fn schedule_delete_prompt(
        &mut self,
        intent: &Intent,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.schedules;
        if !intent.allows(&area.data.page, Action::Delete) {
            return;
        }
        let Some(row) = area.data.page.row(intent.key()) else {
            return;
        };
        let heading = action_label(Action::Delete, is_heartbeat(row.schedule), self.words);
        let words = self.words.schedules();
        let body = words
            .delete_confirm(title(&row.schedule.definition))
            .unwrap_or_else(|_| words.display_unavailable().to_owned());
        let ui = &mut area.ui;
        let unmounted = ui.pending_focus.remove(&Popup::Delete);
        if ui.deleting.take().is_some() || unmounted {
            ui.dialog.dismissed(window, cx);
        }
        ui.deleting = Some(Rc::new(DeletePrompt {
            intent: intent.clone(),
            title: heading.into(),
            body: body.into(),
        }));
        ui.dialog.focus(window, cx);
    }
}
