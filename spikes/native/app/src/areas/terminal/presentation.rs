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
