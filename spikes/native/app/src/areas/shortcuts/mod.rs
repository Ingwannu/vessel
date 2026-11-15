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
//! input method composes in any kit field
//! ([`vessel_app_kit::TextField::focused_composing`]), the listener stands
//! down, so a key the input method needs (Enter to commit a syllable) is
//! never taken for a shortcut.
//!
//! Every app-wide key is here: Find (track C's find bar,
//! `Shell::conversation_find`), the composer's focus, the sidebar and the
//! session's right panel (track F's code panel, `Shell::toggle_code`), and
//! GOAT's two queue keys (track C's `conversation_steer_next` and
//! `conversation_edit_latest_queued`). Keys a control keeps for itself
//! while it has focus (a menu's arrows, a dialog's Escape) stay with it.

use std::time::Instant;

use gpui::{InputPreference, KeystrokeEvent};
use vessel_app_kit::TextField;
use vessel_app_kit::ui::{App, Context, Subscription, Window};
use vessel_app_view::prefs::PreferenceChange;
use vessel_app_view::settings::Section;
use vessel_app_view::shortcuts::{
    Action, Held, KeyPress, Matched, Pending, Platform, ShortcutScope, chord_of, code_of,
    commanding, matched,
};
use vessel_app_view::sidebar::{Step, step_session};

pub(crate) mod page;
#[cfg(test)]
mod tests;

use super::session::Open;
use crate::routes::Route;
use crate::shell::Shell;

/// C2's T1 terminal surface is not integrated yet: the session area
/// draws a placeholder, with no terminal that can hold focus. T1 will
/// replace this seam with its actual focus query when the surface lands.
fn terminal_focused(_shell: &Shell) -> bool {
    false
}

/// What the listener keeps between presses.
pub(crate) struct ShortcutListener {
    /// A sequence's first press (`G` of `G S`), until the next.
    pending: Option<Pending>,
    /// When the app started: a press's time is counted from it.
    since: Instant,
    /// Settings › Shortcuts (its recorder: while a row records, nothing
    /// fires and the key goes to the row).
    pub(crate) page: page::ShortcutsPage,
    /// The interceptor, kept for as long as the window.
    subscription: Option<Subscription>,
    /// The chord held down, to tell the system's repeat from a press.
    held: Held,
}

impl ShortcutListener {
    pub(crate) fn new(cx: &mut App) -> Self {
        Self {
            pending: None,
            since: Instant::now(),
            page: page::ShortcutsPage::new(cx),
            subscription: None,
            held: Held::default(),
        }
    }

    /// Any key came up (the shell's root hears every key-up): the next
    /// press of the same chord is a new press, not a repeat.
    pub(crate) fn released(&mut self) {
        self.held.release();
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
        // The toolkit still calls interceptors for AltGr/text-preferred
        // input. In particular Ctrl+Alt+T must not steal that text.
        if matches!(event.input_preference, InputPreference::CharacterInput)
            || TextField::focused_composing(window, cx)
        {
            self.areas.shortcuts.pending = None;
            return;
        }
        let platform = Platform::native();
        let Some(chord) = chord_of(&press_of(event), platform) else {
            return;
        };
        if self.areas.shortcuts.page.recording() {
            // A recording ends when its page is left (Paseo's
            // `useIsFocused`); until then every key is the row's.
            if self.route == Route::Settings && self.areas.settings.section == Section::Shortcuts {
                self.shortcut_recorded(chord, cx);
                cx.stop_propagation();
                return;
            }
            self.areas.shortcuts.page.capture = None;
        }
        let listener = &mut self.areas.shortcuts;
        let repeat = listener.held.press(&chord);
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
                if action.enabled(self.shortcut_scope(), platform)
                    && self.run_shortcut(action, index, repeat, window, cx)
                {
                    cx.stop_propagation();
                }
            }
        }
    }

    /// The palette's lifetime wins over retained terminal focus.
    fn shortcut_scope(&self) -> ShortcutScope {
        if self.areas.palette.is_open() {
            ShortcutScope::CommandCenter
        } else if terminal_focused(self) {
            ShortcutScope::Terminal
        } else {
            ShortcutScope::Elsewhere
        }
    }

    /// Do what `action` says; whether it did anything (else the key goes
    /// on to the field and the controls). The exhaustive dispatch stays
    /// in one table so adding an action cannot leave a handler implicit.
    fn run_shortcut(
        &mut self,
        action: Action,
        index: Option<u8>,
        repeat: bool,
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
            Action::CycleTheme => {
                let next = self.prefs.settings.appearance.theme.next();
                self.prefer(&PreferenceChange::Theme(next), window, cx);
            }
            Action::GoBoard => self.go(Route::Board, window, cx),
            Action::GoMachines => self.open_settings(Some(Section::Overview), window, cx),
            Action::GoSync => self.open_settings(Some(Section::Sync), window, cx),
            Action::GoDevices => self.open_settings(Some(Section::Devices), window, cx),
            Action::NextSession => return self.step_to(Step::Next, window, cx),
            Action::PreviousSession => return self.step_to(Step::Previous, window, cx),
            Action::JumpSession => return self.jump_to(index, window, cx),
            Action::Find => {
                if self.route != Route::Session {
                    return false;
                }
                self.conversation_find(window, cx);
            }
            Action::FocusComposer => return self.focus_composer(window, cx),
            Action::ToggleSidebar => self.toggle_sidebar(window, cx),
            Action::ToggleInspector => {
                if self.route != Route::Session {
                    return false;
                }
                self.toggle_code(window, cx);
            }
            // GOAT: only in a session, never on a draft, and never for
            // the system's repeat of a held key (one message per press).
            Action::SteerQueued => {
                return !repeat && self.route == Route::Session && self.conversation_steer_next();
            }
            Action::EditQueued => {
                return self.route == Route::Session
                    && self.areas.session.composer.at_start(window, cx)
                    && self.conversation_edit_latest_queued(window, cx);
            }
        }
        true
    }

    /// Give the composer on screen the keys: the session's, or the new
    /// chat's; nothing on a page without one.
    fn focus_composer(&self, window: &mut Window, cx: &mut Context<'_, Self>) -> bool {
        let composer = match self.route {
            Route::Session => &self.areas.session.composer,
            Route::Draft => &self.areas.draft.composer,
            Route::Board | Route::Archive | Route::Pair | Route::Settings => return false,
        };
        composer.focus(window, cx);
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
