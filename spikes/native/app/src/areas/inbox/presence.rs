//! Real input and visibility drive the existing node presence contract.
//! Periodic heartbeats preserve activity age; a socket never becomes activity.
use crate::{routes::Route, shell::Shell};
use vessel_app_kit::ui::{Context, Div, InteractiveElement as _, Window};
use vessel_app_link::{Command, inbox::Ask};
use vessel_app_view::inbox::presence::{Focus, HEARTBEAT_INTERVAL, Report};

pub(super) fn listen(window: &mut Window, cx: &mut Context<'_, Shell>) {
    cx.observe_window_activation(window, |shell, window, cx| {
        shell.inbox_presence(window);
        cx.notify();
    })
    .detach();
    cx.observe_window_visibility(window, |shell, _visibility, window, cx| {
        shell.inbox_presence(window);
        cx.notify();
    })
    .detach();
    cx.spawn_in(window, async move |this, cx| {
        loop {
            cx.background_executor().timer(HEARTBEAT_INTERVAL).await;
            if this
                .update_in(cx, |shell, window, _cx| {
                    shell.inbox_presence(window);
                    if let Some(report) = shell.areas.inbox.presence.tick(std::time::Instant::now())
                    {
                        shell.inbox_presence_report(report);
                    }
                })
                .is_err()
            {
                break;
            }
        }
    })
    .detach();
}
impl Shell {
    /// One frame hook keeps routing changes current without claiming input.
    pub(crate) fn inbox_presence(&mut self, window: &Window) {
        let focus = self.inbox_focus(window);
        if let Some(report) = self
            .areas
            .inbox
            .presence
            .focus(focus, std::time::Instant::now())
        {
            self.inbox_presence_report(report);
        }
    }
    /// Capture before descendant controls can consume a key or pointer press.
    pub(crate) fn inbox_input(content: Div, cx: &Context<'_, Self>) -> Div {
        content
            .capture_key_down(
                cx.listener(|shell, _event, window, _cx| shell.inbox_activity(window)),
            )
            .capture_any_mouse_down(
                cx.listener(|shell, _event, window, _cx| shell.inbox_activity(window)),
            )
            .on_mouse_move(cx.listener(|shell, _event, window, _cx| shell.inbox_activity(window)))
            .on_scroll(cx.listener(|shell, _event, window, _cx| shell.inbox_activity(window)))
    }
    fn inbox_activity(&mut self, window: &Window) {
        if !window.is_window_active() || !window.is_visible() {
            return;
        }
        self.inbox_presence(window);
        if let Some(report) = self.areas.inbox.presence.input(std::time::Instant::now()) {
            self.inbox_presence_report(report);
        }
    }
    pub(super) fn inbox_presence_report(&self, report: Report) {
        for node in self.areas.inbox.store.nodes() {
            if node.listing
                != vessel_app_view::inbox::Listing::Failed(vessel_app_view::inbox::Problem::Offline)
            {
                self.command(Command::Inbox(Ask::Presence {
                    source: node.source.clone(),
                    report,
                }));
            }
        }
    }
    fn inbox_focus(&self, window: &Window) -> Focus {
        if !window.is_window_active() || !window.is_visible() {
            return Focus::Background;
        }
        if self.route == Route::Session
            && let Some(open) = self
                .areas
                .session
                .open
                .as_ref()
                .filter(|open| open.structured)
            && let Some(target) = self
                .areas
                .inbox
                .read_open
                .filter(|target| target.session == open.session)
        {
            return Focus::Session {
                node: target.node,
                session: target.session,
            };
        }
        if self.route == Route::Inbox
            && let Some(key) = self.areas.inbox.detail.key()
        {
            return Focus::Session {
                node: key.node,
                session: key.session,
            };
        }
        Focus::Elsewhere
    }
}
