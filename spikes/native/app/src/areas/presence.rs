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
//! - the shown session, read at each render (`:115-118`);
//! - a heartbeat every 15 s (`:125-148`). The link hears every snapshot
//!   and tells each connected machine; a machine that connects hears the
//!   last one at once.

use std::time::Instant;

use gpui::{
    AnyElement, Context, DispatchPhase, IntoElement, MouseDownEvent, ScrollWheelEvent, Styled as _,
    Subscription, Window,
};
use vessel_app_link::Command;
use vessel_app_view::presence::{HEARTBEAT, Millis, Presence};

use crate::routes::Route;
use crate::shell::Shell;

/// The tracker, its clock and what keeps it fed.
pub(crate) struct PresenceArea {
    presence: Presence,
    /// The clock's zero.
    started: Instant,
    _observers: [Subscription; 3],
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
        }
    }

    fn now(&self) -> Millis {
        Millis::try_from(self.started.elapsed().as_millis()).unwrap_or(Millis::MAX)
    }

    /// Every 15 s, a snapshot to the link, for as long as the shell lives.
    fn heartbeat(cx: &mut Context<'_, Shell>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(HEARTBEAT).await;
                if this.update(cx, |shell, _cx| shell.presence_send()).is_err() {
                    break;
                }
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

    /// The session shown, at each render: a change sends at once.
    pub(crate) fn presence_shown(&mut self) {
        let shown = self
            .areas
            .session
            .open
            .as_ref()
            .filter(|_| self.route == Route::Session)
            .map(|open| (open.machine.clone(), open.session));
        let area = &mut self.areas.presence;
        let now = area.now();
        if area.presence.show(
            shown
                .as_ref()
                .map(|(machine, session)| (machine.as_str(), *session)),
            now,
        ) {
            self.presence_send();
        }
    }

    /// The snapshot to the link. Without a link there is no one to tell.
    fn presence_send(&self) {
        if !self.linked() {
            return;
        }
        let area = &self.areas.presence;
        self.command(Command::Presence(area.presence.snapshot(area.now())));
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
            window.on_mouse_event(move |_event: &ScrollWheelEvent, phase, _window, cx| {
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
