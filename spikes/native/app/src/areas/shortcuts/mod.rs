//! One native key interceptor over view's canonical binding registry. It owns
//! event admission, chord state and capture; feature callbacks own all effects.
//! The default predicates protect editing/terminal/center scopes, while native
//! editing, `CharacterInput` and IME guards remain effective after rebinding.
//!
//! Settings, help and hint chips read the same effective registry. Menu/dialog
//! ownership comes from the kit key-context marker, not inferred open flags.
//! Local control keys remain with their controls whenever no owner handles a
//! shortcut, including an unavailable feature or a missing index payload.

use std::collections::BTreeMap;
use std::time::Instant;

use gpui::{InputPreference, KeystrokeEvent};
use vessel_app_kit::TextField;
use vessel_app_kit::ui::{App, Context, Subscription, Window};
use vessel_app_view::prefs::{ActionId, Bindings};
use vessel_app_view::settings::Section;
use vessel_app_view::shortcuts::context::{
    Admission, CommandCenterState, FocusScope, ShortcutContext,
};
use vessel_app_view::shortcuts::engine::{
    Consume, EffectiveBinding, Engine, KeyRepeat, Resolution,
};
use vessel_app_view::shortcuts::{
    Action, Held, KeyPress, Platform, RegistryError, chord_of, code_of, default_context,
    effective_bindings,
};

mod admission;
mod badges;
mod core;
pub(crate) mod dispatch;
pub(crate) mod help_dialog;
#[cfg(test)]
mod interaction_tests;
mod layout;
pub(crate) mod page;
#[cfg(test)]
mod tests;

use crate::routes::Route;
use crate::shell::Shell;
pub(crate) use admission::book_changed;

/// What the listener keeps between presses.
pub(crate) struct ShortcutListener {
    /// A sequence's first press (`G` of `G S`), until the next.
    engine: Engine,
    compiled: CompiledBindings,
    dispatch: dispatch::NativeDispatcher,
    core_registration: Option<dispatch::Registration>,
    layout_registration: Option<dispatch::Registration>,
    help: help_dialog::HelpDialog,
    /// When the app started: a press's time is counted from it.
    since: Instant,
    /// Settings › Shortcuts (its recorder: while a row records, nothing
    /// fires and the key goes to the row).
    pub(crate) page: page::ShortcutsPage,
    /// The interceptor, kept for as long as the window.
    subscription: Option<Subscription>,
    window_observers: Option<(Subscription, Subscription)>,
    /// The chord held down, to tell the system's repeat from a press.
    held: Held,
    badge_reveal: vessel_app_view::shortcuts::badges::Reveal,
    badge_when: Option<vessel_app_view::shortcuts::context::When>,
    badge_task: Option<gpui::Task<()>>,
}

#[derive(Default)]
struct CompiledBindings {
    overrides: Option<BTreeMap<ActionId, Bindings>>,
    bindings: Vec<EffectiveBinding<Action>>,
}

impl CompiledBindings {
    fn refresh(&mut self, overrides: &BTreeMap<ActionId, Bindings>) -> Result<bool, RegistryError> {
        if self.overrides.as_ref() == Some(overrides) {
            return Ok(false);
        }
        let bindings = effective_bindings(overrides, default_context(Platform::native()))?;
        self.bindings = bindings;
        self.overrides = Some(overrides.clone());
        Ok(true)
    }
}

impl ShortcutListener {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            engine: Engine::default(),
            compiled: CompiledBindings::default(),
            dispatch: dispatch::NativeDispatcher::default(),
            core_registration: None,
            layout_registration: None,
            help: help_dialog::HelpDialog::new(window, cx),
            since: Instant::now(),
            page: page::ShortcutsPage::new(cx),
            subscription: None,
            window_observers: None,
            held: Held::default(),
            badge_reveal: vessel_app_view::shortcuts::badges::Reveal::default(),
            badge_when: None,
            badge_task: None,
        }
    }

    /// Any key came up (the shell's root hears every key-up): the next
    /// press of the same chord is a new press, not a repeat.
    pub(crate) fn released(&mut self) {
        self.held.release();
    }
}

/// Start listening: once, as the shell starts.
pub(crate) fn listen(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    if let Some(old) = shell.areas.shortcuts.core_registration.take() {
        shell.remove_shortcut_handler(&old);
    }
    match dispatch::register_core(shell) {
        Ok(registration) => shell.areas.shortcuts.core_registration = Some(registration),
        Err(error) => tracing::warn!(%error, "shortcut core registration refused"),
    }
    if let Some(old) = shell.areas.shortcuts.layout_registration.take() {
        shell.remove_shortcut_handler(&old);
    }
    match layout::register(shell) {
        Ok(registration) => shell.areas.shortcuts.layout_registration = Some(registration),
        Err(error) => tracing::warn!(%error, "layout shortcut registration refused"),
    }
    let activation = cx.observe_window_activation(window, |shell, window, _cx| {
        if !window.is_window_active() {
            shell.areas.shortcuts.engine.reset();
            shell.areas.shortcuts.held.release();
            badges::clear(&mut shell.areas.shortcuts);
        }
    });
    let visibility = cx.observe_window_visibility(window, |shell, visibility, _window, _cx| {
        if !visibility.is_visible() {
            shell.areas.shortcuts.engine.reset();
            shell.areas.shortcuts.held.release();
            badges::clear(&mut shell.areas.shortcuts);
        }
    });
    shell.areas.shortcuts.window_observers = Some((activation, visibility));
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
    // GPUI separates typed text (key_char, e.g. Option-S's ß) from the
    // platform-normalized base key. Keep text first, then derive only the
    // fallback identity that GPUI actually supplied; never invent a scan code.
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
        // S2: this guard precedes recorder, matcher and actions. Ctrl+Alt text
        // and an IME commit must propagate and must cancel any pending chord.
        if matches!(event.input_preference, InputPreference::CharacterInput)
            || TextField::focused_composing(window, cx)
        {
            self.areas.shortcuts.engine.reset();
            badges::clear(&mut self.areas.shortcuts);
            return;
        }
        // Visibility is independent of viewport availability: a hidden wide
        // window must neither run a command nor append to Settings capture.
        if !window.is_visible() {
            self.areas.shortcuts.engine.reset();
            self.areas.shortcuts.held.release();
            badges::clear(&mut self.areas.shortcuts);
            return;
        }
        if !self.global_shortcuts_enabled() {
            book_changed(self);
            return;
        }
        let keyboard_available = available(window);
        if !keyboard_available {
            self.areas.shortcuts.engine.reset();
            badges::clear(&mut self.areas.shortcuts);
            self.areas.shortcuts.page.leave();
        }
        let context = self.shortcut_context(event, window, cx);
        if context.admission == Admission::Overlay {
            self.areas.shortcuts.engine.reset();
            self.areas.shortcuts.held.release();
            badges::clear(&mut self.areas.shortcuts);
            return;
        }
        if !keyboard_available {
            return;
        }
        let press = press_of(event);
        if self.areas.shortcuts.page.recording() {
            self.areas.shortcuts.engine.reset();
            if self.route == Route::Settings && self.areas.settings.section == Section::Shortcuts {
                if let vessel_app_view::shortcuts::input::InputConversion::Input(input) =
                    vessel_app_view::shortcuts::input::ShortcutInput::from_press(
                        &press,
                        Platform::native(),
                    )
                    && let Some(chord) = input.recording_chord()
                {
                    self.shortcut_recorded(chord, cx);
                    cx.stop_propagation();
                }
                return;
            }
            self.areas.shortcuts.page.capture = None;
        }
        if self.command_center_owns_key(event) {
            self.areas.shortcuts.engine.reset();
            self.areas.shortcuts.held.release();
            return;
        }
        self.resolve_shortcut(&press, context, window, cx);
    }

    /// Admission and local controls have already kept their input. This is
    /// still the same engine/cache/dispatcher, with no second event listener.
    fn resolve_shortcut(
        &mut self,
        press: &KeyPress,
        context: ShortcutContext,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let listener = &mut self.areas.shortcuts;
        match listener.compiled.refresh(&self.prefs.settings.shortcuts) {
            Ok(true) => {
                listener.engine.reset();
                listener.held.release();
                badges::clear(listener);
            }
            Ok(false) => {}
            Err(error) => {
                listener.engine.reset();
                tracing::warn!(%error, "shortcut registry refused");
                return;
            }
        }
        let repeat =
            if chord_of(press, context.platform).is_some_and(|chord| listener.held.press(&chord)) {
                KeyRepeat::Repeated
            } else {
                KeyRepeat::Initial
            };
        let found = listener.engine.resolve_keypress(
            press,
            repeat,
            listener.since.elapsed(),
            context,
            &listener.compiled.bindings,
        );
        match found {
            Resolution::Pass => {}
            Resolution::Waiting => cx.stop_propagation(),
            Resolution::Run {
                action,
                index,
                prevent_default: _,
                stop_propagation,
            } => {
                let request = dispatch::ShortcutRequest {
                    action,
                    scope: action.scope(),
                    index,
                    context,
                    repeat,
                };
                if self.dispatch_shortcut_to_owners(request, window, cx)
                    && stop_propagation == Consume::Yes
                {
                    cx.stop_propagation();
                }
            }
        }
    }

    fn shortcut_context(
        &self,
        event: &KeystrokeEvent,
        window: &Window,
        cx: &App,
    ) -> ShortcutContext {
        let mut context = default_context(Platform::native());
        context.command_center = if self.areas.palette.is_open() {
            CommandCenterState::Open
        } else {
            CommandCenterState::Closed
        };
        if event
            .context_stack
            .iter()
            .any(|entry| entry.contains(vessel_app_kit::popup::KEY_CONTEXT))
            || self.areas.shortcuts.help.is_open()
        {
            context.admission = Admission::Overlay;
        }
        let composer_focused = match self.route {
            Route::Session => self
                .areas
                .session
                .composer
                .focus_handle(cx)
                .is_focused(window),
            Route::Draft => self
                .areas
                .draft
                .composer
                .focus_handle(cx)
                .is_focused(window),
            Route::Board | Route::Archive | Route::Pair | Route::Settings => false,
        };
        context.focus = if self.areas.palette.is_open() {
            FocusScope::CommandCenter
        } else if self.terminal_focused(window) {
            FocusScope::Terminal
        } else if composer_focused {
            FocusScope::MessageInput
        } else if event
            .context_stack
            .iter()
            .any(|entry| entry.contains("Input"))
        {
            FocusScope::Editable
        } else {
            FocusScope::Other
        };
        context
    }
}

/// The same availability rule hides chips, disables capture and gates the listener.
pub(crate) fn available(window: &Window) -> bool {
    !cfg!(any(target_os = "android", target_os = "ios"))
        && f32::from(window.viewport_size().width) >= vessel_app_kit::picker::COMPACT
}

/// A route/Settings section change cancels the recorder before another key arrives.
pub(crate) fn navigation_changed(shell: &mut Shell) {
    shell.areas.shortcuts.engine.reset();
    if shell.route != Route::Settings || shell.areas.settings.section != Section::Shortcuts {
        shell.areas.shortcuts.page.leave();
    }
}

/// Opening a modal cancels a recorder/prefix before the new focused field can
/// receive a key. This is used by the existing palette and shortcut-help openers.
pub(crate) fn modal_opened(shell: &mut Shell) {
    shell.areas.shortcuts.engine.reset();
    shell.areas.shortcuts.held.release();
    badges::clear(&mut shell.areas.shortcuts);
    shell.areas.shortcuts.page.leave();
}

/// Preference writes cancel an old hold before another modifier/key arrives.
pub(crate) fn preferences_changed(shell: &mut Shell) {
    shell.areas.shortcuts.engine.reset();
    shell.areas.shortcuts.held.release();
    shell.areas.shortcuts.compiled.overrides = None;
    badges::clear(&mut shell.areas.shortcuts);
}
