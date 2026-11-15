//! Cursor work exists only for the currently focused, presented terminal.
//! Event subscriptions restart it; holding motion never leaves a polling timer.

use super::{BLINK, Blink, Phase};
use crate::{routes::Route, shell::Shell};
use vessel_app_kit::ui::{Context, Window};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Whether a presented terminal is entitled to schedule cursor work.
pub(super) enum Playback {
    Blinking,
    Steady,
}

impl Shell {
    /// Keyboard context follows the currently focused terminal opening, not a
    /// session selection or a guessed loading/readiness phase.
    pub(crate) fn terminal_focused(&self, window: &Window) -> bool {
        self.route == Route::Session
            && self.areas.terminal.shown.is_some()
            && self.areas.terminal.focus.is_focused(window)
    }

    /// The grid, rather than the hidden conversation composer, owns terminal input focus.
    pub(crate) fn focus_terminal(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.areas.terminal.focus.focus(window, cx);
        self.terminal_present(window, cx);
    }

    /// Called by the shell's render and focus/visibility subscriptions, so
    /// preference changes and navigation do not depend on incoming output.
    pub(crate) fn terminal_present(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.areas.terminal.observers.is_empty() {
            let focus = self.areas.terminal.focus.clone();
            self.areas.terminal.observers = vec![
                cx.observe_window_activation(window, Self::terminal_present),
                cx.observe_window_visibility(window, |shell, _, window, cx| {
                    shell.terminal_present(window, cx);
                }),
                cx.on_focus_in(&focus, window, Self::terminal_present),
                cx.on_focus_out(&focus, window, |shell, _, window, cx| {
                    shell.terminal_present(window, cx);
                }),
            ];
        }
        let area = &mut self.areas.terminal;
        let playback = if self.route == Route::Session
            && window.is_visible()
            && window.is_window_active()
            && area.focus.is_focused(window)
            && matches!(area.phase, Phase::Ready | Phase::Unread)
            && !cx.reduce_motion()
        {
            Playback::Blinking
        } else {
            Playback::Steady
        };
        area.playback = playback;
        match playback {
            Playback::Steady => {
                area.blinking = None;
                area.blink = Blink::On;
            }
            Playback::Blinking => self.terminal_blink(cx),
        }
        // This invalidates scheme/font changes even when geometry and motion hold.
        self.terminal_draw(cx);
    }

    pub(super) fn terminal_blink(&mut self, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.terminal;
        if area.blinking.is_some() {
            return;
        }
        let Some(opening) = area.shown.clone() else {
            return;
        };
        area.blinking = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(BLINK).await;
                let going = this.update(cx, |shell, cx| {
                    let area = &mut shell.areas.terminal;
                    if !area.owns(&opening) {
                        return false;
                    }
                    if cx.reduce_motion() || area.playback == Playback::Steady {
                        area.blinking = None;
                        area.blink = Blink::On;
                        shell.terminal_draw(cx);
                        return false;
                    }
                    area.blink = match area.blink {
                        Blink::On => Blink::Off,
                        Blink::Off => Blink::On,
                    };
                    shell.terminal_draw(cx);
                    true
                });
                if !matches!(going, Ok(true)) {
                    break;
                }
            }
        }));
    }
}

#[cfg(test)]
mod focus_tests {
    use super::*;
    use gpui::TestAppContext;
    use vessel_app_link::TerminalOpening;
    use vessel_proto::SessionId;

    #[gpui::test]
    fn focus_query_tracks_the_opening_focus_route_and_close(cx: &mut TestAppContext) {
        crate::shell::language_tests::init(cx);
        let (shell, cx) = cx.add_window_view(Shell::without_io);
        shell.update_in(cx, |shell, window, cx| {
            shell.route = Route::Session;
            shell.areas.terminal.shown = Some(TerminalOpening::new(
                "fake".parse().unwrap(),
                SessionId::random(),
            ));
            shell.areas.terminal.focus.focus(window, cx);
            assert!(shell.terminal_focused(window));
            shell.areas.session.composer.focus(window, cx);
            assert!(!shell.terminal_focused(window));
            shell.areas.terminal.focus.focus(window, cx);
            shell.route = Route::Board;
            assert!(!shell.terminal_focused(window));
            shell.route = Route::Session;
            assert!(shell.terminal_focused(window));
            shell.terminal_closed();
            assert!(!shell.terminal_focused(window));
        });
    }
}
