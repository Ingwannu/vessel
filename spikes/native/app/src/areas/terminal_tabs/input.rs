//! What a person types into a terminal tab: a named key or a Ctrl or Alt
//! chord as Paseo's encoding (`view::terminal::encode_key`), text and an
//! IME's commit through the platform's text input, a paste (Paseo:
//! Cmd+V on macOS, Ctrl+V elsewhere; Ctrl+Shift+V too, as terminals do),
//! all through the pane's one sender, in order.
//!
//! [Decision Log]
//! - Purpose: Paseo's xterm takes keys through a hidden textarea, so an
//!   IME composes Hangul in it; its native renderer has a hidden input.
//! - Constraint: GPUI gives composed text only to an `EntityInputHandler`
//!   painted for the focused element; key-down listeners run first and,
//!   when they stop the key, no text is inserted.
//! - Choice: the key-down listener takes what has no text (named keys,
//!   chords) and stops it; everything that types text goes on to a small
//!   entity per pane ([`TerminalKeys`]) that forwards commits and the
//!   composing text to the shell. The composing text is drawn at the
//!   cursor (`body`), as xterm draws its textarea there.
//! - Cost: Command chords are left to the app's shortcuts (Paseo's too).

use std::ops::Range;

use gpui::{EntityInputHandler, KeyDownEvent, UTF16Selection};
use vessel_app_kit::ui::{Bounds, Context, Pixels, WeakEntity, Window};
use vessel_app_view::terminal::{
    Function, Key, Mods, Named, Taken, Typed, encode_key, encode_paste,
};

use super::PaneKey;
use crate::shell::Shell;

/// The platform's text input for one pane.
pub(crate) struct TerminalKeys {
    shell: WeakEntity<Shell>,
    key: PaneKey,
    /// The IME's text being composed (UTF-16 length is its marked range).
    marked: Option<String>,
}

impl TerminalKeys {
    pub(super) fn new(shell: WeakEntity<Shell>, key: PaneKey) -> Self {
        Self {
            shell,
            key,
            marked: None,
        }
    }

    fn tell(
        &self,
        cx: &mut Context<'_, Self>,
        act: impl FnOnce(&mut Shell, &PaneKey, &mut Context<'_, Shell>),
    ) {
        let key = self.key.clone();
        drop(self.shell.update(cx, |shell, cx| act(shell, &key, cx)));
    }
}

impl EntityInputHandler for TerminalKeys {
    fn text_for_range(
        &mut self,
        _range: Range<usize>,
        _adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<'_, Self>,
    ) -> Option<String> {
        // A terminal has no editable text around the cursor.
        None
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<'_, Self>,
    ) -> Option<UTF16Selection> {
        let end = self
            .marked
            .as_deref()
            .map_or(0, |text| text.encode_utf16().count());
        Some(UTF16Selection {
            range: end..end,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<'_, Self>,
    ) -> Option<Range<usize>> {
        self.marked
            .as_deref()
            .map(|text| 0..text.encode_utf16().count())
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        self.marked = None;
        self.tell(cx, |shell, key, cx| {
            shell.terminal_tab_preedit(key, None, cx);
        });
    }

    fn replace_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.marked = None;
        let text = text.to_owned();
        self.tell(cx, move |shell, key, cx| {
            shell.terminal_tab_preedit(key, None, cx);
            shell.terminal_tab_type(key, &text, cx);
        });
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        new_text: &str,
        _new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.marked = (!new_text.is_empty()).then(|| new_text.to_owned());
        let marked = self.marked.clone();
        self.tell(cx, move |shell, key, cx| {
            shell.terminal_tab_preedit(key, marked, cx);
        });
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Option<Bounds<Pixels>> {
        // The candidate window opens at the terminal's cursor.
        let shell = self.shell.upgrade()?;
        let app: &vessel_app_kit::ui::App = cx;
        Some(
            shell
                .read(app)
                .terminal_tab_cursor(&self.key, element_bounds, app)
                .unwrap_or(element_bounds),
        )
    }

    fn character_index_for_point(
        &mut self,
        _point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<'_, Self>,
    ) -> Option<usize> {
        None
    }
}

/// A GPUI key name as a named terminal key.
fn named(key: &str) -> Option<Named> {
    Some(match key {
        "enter" => Named::Enter,
        "tab" => Named::Tab,
        "backspace" => Named::Backspace,
        "escape" => Named::Escape,
        "up" => Named::Up,
        "down" => Named::Down,
        "left" => Named::Left,
        "right" => Named::Right,
        "home" => Named::Home,
        "end" => Named::End,
        "insert" => Named::Insert,
        "delete" => Named::Delete,
        "pageup" => Named::PageUp,
        "pagedown" => Named::PageDown,
        other => {
            let number = other.strip_prefix('f')?.parse::<u8>().ok()?;
            Named::F(Function::new(number)?)
        }
    })
}

/// A GPUI key name as the one character a chord sends with it.
fn character(key: &str) -> Option<char> {
    if key == "space" {
        return Some(' ');
    }
    let mut chars = key.chars();
    let only = chars.next()?;
    chars.next().is_none().then_some(only)
}

fn mods_of(modifiers: gpui::Modifiers) -> Mods {
    let mut mods = Mods::NONE;
    for (held, which) in [
        (modifiers.shift, Mods::SHIFT),
        (modifiers.alt, Mods::ALT),
        (modifiers.control, Mods::CTRL),
    ] {
        if held {
            mods = mods.with(which);
        }
    }
    mods
}

/// Paseo's paste chord: Cmd+V on macOS, Ctrl+V elsewhere (and the
/// terminals' Ctrl+Shift+V).
fn is_paste(stroke: &gpui::Keystroke) -> bool {
    let modifiers = stroke.modifiers;
    stroke.key == "v"
        && !modifiers.alt
        && if cfg!(target_os = "macos") {
            modifiers.platform && !modifiers.control
        } else {
            modifiers.control && !modifiers.platform
        }
}

impl Shell {
    /// A key down in a focused terminal tab.
    pub(super) fn terminal_tab_key(
        &mut self,
        key: &PaneKey,
        event: &KeyDownEvent,
        cx: &mut Context<'_, Self>,
    ) {
        let stroke = &event.keystroke;
        if is_paste(stroke) {
            self.terminal_tab_paste(key, cx);
            cx.stop_propagation();
            return;
        }
        let modifiers = stroke.modifiers;
        // Command chords are the app's; AltGr text (Ctrl+Alt with a typed
        // character) goes on to the text input.
        if modifiers.platform || (modifiers.control && modifiers.alt && stroke.key_char.is_some()) {
            return;
        }
        let Some(pane) = self.areas.terminal_tabs.panes.get(key) else {
            return;
        };
        let cursor = pane
            .model
            .as_ref()
            .map(|model| model.grid().cursor_keys())
            .unwrap_or_default();
        let mods = mods_of(modifiers);
        let sequence = if let Some(named) = named(&stroke.key) {
            encode_key(Key::Named(named), mods, cursor)
        } else if (modifiers.control || modifiers.alt)
            && let Some(chord) = character(&stroke.key)
        {
            encode_key(Key::Char(chord), mods, cursor)
        } else {
            // Text: the input handler receives it.
            return;
        };
        self.terminal_tab_type(key, &sequence, cx);
        cx.stop_propagation();
    }

    /// Keys typed (or committed by the IME) into a tab.
    pub(super) fn terminal_tab_type(
        &mut self,
        key: &PaneKey,
        text: &str,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        if text.is_empty() {
            return;
        }
        pane.typed = Typed::Once;
        // Typing returns the view to the live screen, as xterm does.
        pane.viewport = vessel_app_view::terminal::Viewport::default();
        if pane.sender.keys(text) == Taken::DroppedFirst {
            tracing::warn!("a terminal tab's input overflowed; keys were dropped");
        }
        self.terminal_tab_pump(key);
        self.terminal_tab_draw(key, cx);
    }

    /// The clipboard's text pasted into a tab, bracketed when its program
    /// asked.
    fn terminal_tab_paste(&mut self, key: &PaneKey, cx: &mut Context<'_, Self>) {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };
        let Some(pane) = self.areas.terminal_tabs.panes.get(key) else {
            return;
        };
        let bracketed = pane
            .model
            .as_ref()
            .map(|model| model.grid().bracketed())
            .unwrap_or_default();
        self.terminal_tab_type(key, &encode_paste(&text, bracketed), cx);
    }

    /// The IME's composing text, drawn at the cursor.
    pub(super) fn terminal_tab_preedit(
        &mut self,
        key: &PaneKey,
        text: Option<String>,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key)
            && pane.preedit != text
        {
            pane.preedit = text;
            cx.notify();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{character, mods_of, named};
    use vessel_app_view::terminal::{Function, Mods, Named};

    #[test]
    fn gpui_key_names_map_to_named_keys() {
        assert_eq!(named("enter"), Some(Named::Enter));
        assert_eq!(named("pagedown"), Some(Named::PageDown));
        assert_eq!(named("f12"), Some(Named::F(Function::new(12).unwrap())));
        assert_eq!(named("f13"), None);
        assert_eq!(named("a"), None);
        assert_eq!(named("space"), None);
        assert_eq!(character("space"), Some(' '));
        assert_eq!(character("c"), Some('c'));
        assert_eq!(character("enter"), None);
    }

    #[test]
    fn held_modifiers_become_one_set() {
        let modifiers = gpui::Modifiers {
            control: true,
            shift: true,
            ..gpui::Modifiers::default()
        };
        let mods = mods_of(modifiers);
        assert!(mods.has(Mods::CTRL) && mods.has(Mods::SHIFT) && !mods.has(Mods::ALT));
    }
}
