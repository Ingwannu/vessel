//! This window's presence on every connected machine (`push.presence`;
//! `vessel_app_view::presence` holds the Decision Log and Paseo's
//! references), wired as Paseo's `use-client-activity.ts` wires its
//! tracker:
//! - input: a key (`observe_keystrokes`), a pointer press or a wheel
//!   anywhere in the window (a capture-phase listener painted at the root,
//!   so no element stops it first), and the window becoming active, as
//!   Paseo's window `focus` (`:64-67,78-82`);
//! - visibility: the window shown or not (`observe_window_visibility`), as
//!   Paseo reads the document's (`:69-75`);
//! - the shown session, read at each render (`:115-118`), or the terminal
//!   whose tab has the focused pane there (claims row 3);
//! - whether the window notifies, read at each render, as the page reads
//!   Settings' alerts and the browser's permission (ADR 0089, "Turned on",
//!   item 5);
//! - a heartbeat at once and then every 15 s (`:125-148`): the first one
//!   goes as soon as the shell exists, so opening reaches the link before
//!   the first machine connects, and a machine that connects hears that
//!   last snapshot at once (app review R1). The link hears every snapshot
//!   and tells each connected machine.

use std::time::Instant;

use gpui::{
    AnyElement, Context, DispatchPhase, IntoElement, MouseDownEvent, ScrollEvent, Styled as _,
    Subscription, Window,
};
use vessel_app_view::presence::{HEARTBEAT, Millis, Presence, Shown};

use crate::routes::Route;
use crate::shell::Shell;

/// The tracker, its clock and what keeps it fed.
pub(crate) struct PresenceArea {
    presence: Presence,
    /// The clock's zero.
    started: Instant,
    _observers: [Subscription; 3],
    /// Every snapshot handed to the link, for the tests.
    #[cfg(test)]
    handed: Vec<vessel_app_view::presence::Snapshot>,
}

impl std::fmt::Debug for PresenceArea {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PresenceArea")
            .field("presence", &self.presence)
            .finish_non_exhaustive()
    }
}

impl PresenceArea {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        let visibility = cx.observe_window_visibility(window, |shell, visibility, _window, cx| {
            shell.presence_visibility(visibility.is_visible(), cx);
        });
        let activation = cx.observe_window_activation(window, |shell, window, cx| {
            if window.is_window_active() {
                shell.presence_input(cx);
            }
        });
        let keys = cx.observe_keystrokes(|shell, _event, _window, cx| shell.presence_input(cx));
        Self::heartbeat(cx);
        Self {
            presence: Presence::new(0, true),
            started: Instant::now(),
            _observers: [visibility, activation, keys],
            #[cfg(test)]
            handed: Vec::new(),
        }
    }

    fn now(&self) -> Millis {
        Millis::try_from(self.started.elapsed().as_millis()).unwrap_or(Millis::MAX)
    }

    /// A snapshot to the link at once, then every 15 s, for as long as the
    /// shell lives. Paseo's `start()` sends before its interval
    /// (`use-client-activity.ts:129-132`), and so does the page at mount.
    fn heartbeat(cx: &mut Context<'_, Shell>) {
        cx.spawn(async move |this, cx| {
            loop {
                if this.update(cx, |shell, _cx| shell.presence_send()).is_err() {
                    break;
                }
                cx.background_executor().timer(HEARTBEAT).await;
            }
        })
        .detach();
    }
}

impl Shell {
    /// Real input: recorded, then a heartbeat unless one went in the last
    /// 5 s (Paseo's `maybeSendImmediateHeartbeat`).
    pub(crate) fn presence_input(&mut self, _cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.presence;
        let now = area.now();
        area.presence.input(now);
        if area.presence.immediate(now) {
            self.presence_send();
        }
    }

    fn presence_visibility(&mut self, visible: bool, _cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.presence;
        let now = area.now();
        // Hidden waits for the next heartbeat; shown again is input and
        // tries an immediate one (Paseo's `:69-75`).
        if area.presence.visibility(visible, now) && visible && area.presence.immediate(now) {
            self.presence_send();
        }
    }

    /// The session shown, or the terminal whose tab has the focused pane
    /// in its workspace, at each render: a change sends at once.
    pub(crate) fn presence_shown(&mut self) {
        let shown = self
            .areas
            .session
            .open
            .as_ref()
            .filter(|_| self.route == Route::Session)
            .map(|open| (open.machine.clone(), Shown::Session(open.session)));
        let shown = shown.map(|(machine, session)| {
            let terminal = crate::areas::layout::current_key(self)
                .ok()
                .and_then(|key| {
                    crate::areas::layout::read_layout(
                        self,
                        &key,
                        crate::areas::terminal_tabs::shown_terminal,
                    )
                });
            (machine, terminal.map_or(session, Shown::Terminal))
        });
        let notifies = self.window_notifies();
        let area = &mut self.areas.presence;
        let now = area.now();
        let visible = area.presence.visible();
        let moved = area.presence.show(
            shown
                .as_ref()
                .map(|(machine, what)| (machine.as_str(), *what)),
            now,
        );
        // Settings' alerts or the system's service changed whether the
        // window notifies: the machines hear it at once (ADR 0089, "Turned
        // on", item 5).
        if area.presence.notify(notifies) || moved {
            self.presence_send();
        }
        if let Some((machine, Shown::Terminal(terminal))) = shown.filter(|_| visible) {
            self.terminal_attention_seen(&machine, terminal);
        }
    }

    /// The snapshot to the link.
    fn presence_send(&mut self) {
        let area = &mut self.areas.presence;
        let snapshot = area.presence.snapshot(area.now());
        #[cfg(test)]
        area.handed.push(snapshot.clone());
        self.report_presence(snapshot);
    }
}

/// A listener painted at the window's root for pointer presses and wheels,
/// in the capture phase, so an element that stops either still counts it.
pub(crate) fn observer(cx: &Context<'_, Shell>) -> AnyElement {
    let shell = cx.weak_entity();
    gpui::canvas(
        |_, _, _| (),
        move |_bounds, (), window, _cx| {
            let pressed = shell.clone();
            window.on_mouse_event(move |_event: &MouseDownEvent, phase, _window, cx| {
                if phase == DispatchPhase::Capture {
                    drop(pressed.update(cx, Shell::presence_input));
                }
            });
            let scrolled = shell.clone();
            window.on_mouse_event(move |_event: &ScrollEvent, phase, _window, cx| {
                if phase == DispatchPhase::Capture {
                    drop(scrolled.update(cx, Shell::presence_input));
                }
            });
        },
    )
    .absolute()
    .size_0()
    .into_any_element()
}

#[cfg(test)]
mod tests;
