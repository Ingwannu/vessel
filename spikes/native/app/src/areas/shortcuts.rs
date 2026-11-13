//! The app's one key listener (DESIGN.md §7.1; the page's
//! `features/shortcuts/listener.ts`) and the shortcuts the shell answers
//! (its `features/shell/use-shell-shortcuts.ts`, `use-new-session.ts` and
//! the palette's): a key press becomes a chord, the chord an action with
//! the person's overrides (`view::shortcuts`), and the action runs here.
//!
//! The rules are the page's: a chord with Mod, Ctrl, Alt or Meta fires
//! while typing in a field, one without (`N`, `/`, `G S`) stands down; no
//! key fires while Settings › Shortcuts records one. The listener hears a
//! key before anything else does (GPUI's keystroke interceptor), so an
//! action that acts takes the key from the field and the controls (the
//! page's `preventDefault`); one that does not leaves it to them. While an
//! input method composes, the system's input method holds the keys and
//! none reaches the listener.
//!
//! Find (track C's find bar keeps its own key for now), the composer's
//! focus and the two panels have no handler here yet: their keys pass on.

use std::time::Instant;

use gpui::KeystrokeEvent;
use vessel_app_kit::ui::{Context, Subscription, Window};
use vessel_app_view::settings::Section;
use vessel_app_view::shortcuts::{
    Action, KeyPress, Matched, Pending, Platform, chord_of, code_of, commanding, matched,
};
use vessel_app_view::sidebar::{Step, step_session};

use super::session::Open;
use crate::routes::Route;
use crate::shell::Shell;

/// What the listener keeps between presses.
pub(crate) struct ShortcutListener {
    /// A sequence's first press (`G` of `G S`), until the next.
    pending: Option<Pending>,
    /// When the app started: a press's time is counted from it.
    since: Instant,
    /// Settings › Shortcuts is recording a key: nothing fires. Its
    /// recorder sets and clears it.
    pub(crate) recording: bool,
    /// The interceptor, kept for as long as the window.
    subscription: Option<Subscription>,
}

impl ShortcutListener {
    pub(crate) fn new() -> Self {
        Self {
            pending: None,
            since: Instant::now(),
            recording: false,
            subscription: None,
        }
    }

    /// Milliseconds since the app started.
    fn now(&self) -> u64 {
        u64::try_from(self.since.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

/// Start listening: once, as the shell starts.
pub(crate) fn listen(shell: &mut Shell, cx: &mut Context<'_, Shell>) {
    let this = cx.weak_entity();
    let subscription = cx.intercept_keystrokes(move |event, window, cx| {
        drop(this.update(cx, |shell, cx| shell.shortcut(event, window, cx)));
    });
    shell.areas.shortcuts.subscription = Some(subscription);
}

/// A toolkit keystroke as the press `view::shortcuts` reads.
fn press_of(event: &KeystrokeEvent) -> KeyPress {
    let stroke = &event.keystroke;
    let modifiers = stroke.modifiers;
    KeyPress {
        key: stroke
            .key_char
            .clone()
            .unwrap_or_else(|| stroke.key.clone()),
        code: code_of(&stroke.key),
        ctrl: modifiers.control,
        alt: modifiers.alt,
        shift: modifiers.shift,
        meta: modifiers.platform,
    }
}

impl Shell {
    fn shortcut(
        &mut self,
        event: &KeystrokeEvent,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.areas.shortcuts.recording {
            return;
        }
        let platform = Platform::native();
        let Some(chord) = chord_of(&press_of(event), platform) else {
            return;
        };
        let listener = &mut self.areas.shortcuts;
        let typing = event
            .context_stack
            .iter()
            .any(|context| context.contains("Input"));
        if typing && !commanding(&chord) {
            listener.pending = None;
            return;
        }
        let found = matched(
            &chord,
            listener.pending.take().as_ref(),
            &self.prefs.settings.shortcuts,
            listener.now(),
            platform,
        );
        match found {
            Matched::Wait(pending) => self.areas.shortcuts.pending = Some(pending),
            Matched::None => {}
            Matched::Run { action, index } => {
                if self.run_shortcut(action, index, window, cx) {
                    cx.stop_propagation();
                }
            }
        }
    }

    /// Do what `action` says; whether it did anything (else the key goes
    /// on to the field and the controls).
    fn run_shortcut(
        &mut self,
        action: Action,
        index: Option<u8>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        match action {
            Action::Palette => self.open_palette(window, cx),
            Action::NewSession => {
                let machine = self.machine_in_view();
                self.go(Route::Draft, window, cx);
                if let Some(machine) = machine {
                    self.choose_machine(machine, window, cx);
                }
            }
            Action::Settings => self.open_settings(None, window, cx),
            Action::GoBoard => self.go(Route::Board, window, cx),
            Action::GoMachines => self.open_settings(Some(Section::Machines), window, cx),
            Action::GoSync => self.open_settings(Some(Section::Sync), window, cx),
            Action::GoDevices => self.open_settings(Some(Section::Devices), window, cx),
            Action::NextSession => return self.step_to(Step::Next, window, cx),
            Action::PreviousSession => return self.step_to(Step::Previous, window, cx),
            Action::JumpSession => return self.jump_to(index, window, cx),
            Action::Find
            | Action::FocusComposer
            | Action::ToggleSidebar
            | Action::ToggleInspector => return false,
        }
        true
    }

    /// The machine the window shows: the open session's, or the one the
    /// board is scoped to (the page's `machineInView`).
    pub(super) fn machine_in_view(&self) -> Option<String> {
        match self.route {
            Route::Session => self
                .areas
                .session
                .open
                .as_ref()
                .map(|open| open.machine.clone()),
            Route::Board | Route::Archive => self.board_scope().map(str::to_owned),
            Route::Draft | Route::Pair | Route::Settings => None,
        }
    }

    /// The session on screen, if one is.
    fn session_in_view(&self) -> Option<(&str, vessel_proto::SessionId)> {
        (self.route == Route::Session)
            .then_some(self.areas.session.open.as_ref())
            .flatten()
            .map(|open| (open.machine.as_str(), open.session))
    }

    /// Open the session one `step` from the one on screen, in the
    /// sidebar's order.
    fn step_to(&mut self, step: Step, window: &mut Window, cx: &mut Context<'_, Self>) -> bool {
        let order = self.fleet.sidebar_order(self.prefs.settings.sidebar);
        let open = step_session(&order, self.session_in_view(), step).map(Open::of);
        open.is_some_and(|open| {
            self.open_session(open, window, cx);
            true
        })
    }

    /// Open the sidebar's session `index` (1 to 9).
    fn jump_to(
        &mut self,
        index: Option<u8>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let order = self.fleet.sidebar_order(self.prefs.settings.sidebar);
        let open = index
            .and_then(|index| usize::from(index).checked_sub(1))
            .and_then(|at| order.get(at).copied())
            .map(Open::of);
        open.is_some_and(|open| {
            self.open_session(open, window, cx);
            true
        })
    }
}
