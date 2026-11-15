//! What the composer's menus, recall, drafts and attachments need of its
//! text engine: the caret and the selection, a word replaced in place, a
//! call after every edit, the keys a menu takes before the text does, and
//! pasted images or files.
//!
//! [Decision Log]
//! - Purpose: Paseo's composer completes `/` commands and `@` files from
//!   a menu at the caret (`hooks/use-autocomplete.ts`: Up, Down, Enter,
//!   Tab and Escape drive the menu while it shows), recalls sent messages
//!   with Up and Down (`composer/recall.ts`), and takes pasted images as
//!   attachments (`composer/attachments/`). The app's areas cannot name the
//!   text engine's actions (only `main.rs` and the kit may name the
//!   component layer).
//! - Constraint: the engine binds those keys to actions (`MoveUp`,
//!   `MoveDown`, `Enter`, `IndentInline`, `Escape`, `Paste`) in its own
//!   context, and GPUI runs a binding's action before any key listener,
//!   so a menu's `capture_key_down` above the text never sees them in time.
//! - Alternatives: a keystroke interceptor for the whole window (it would
//!   have to know which field has the keys, and runs for every key in the
//!   app); a second text engine for the composer (the kit has one, with
//!   its input-method and focus behaviour, and keeps it).
//! - Choice: [`hooked`] wraps the composer's element and captures those
//!   actions on their way down to the text: the screen's handler answers
//!   [`KeyAnswer::Taken`] (the text never sees it) or [`KeyAnswer::Pass`]
//!   (everything happens as before: Enter sends, Tab indents, the arrows
//!   move the caret). While an input method composes, the handler is not
//!   asked: the key is the input method's. A paste is offered only when
//!   the clipboard holds images or files; a text paste is stopped only when
//!   the handler takes them.
//! - Cost: the handler runs once per such key while the composer has the
//!   keys; a key a screen takes by mistake is lost to the text, so screens
//!   take a key only while their menu shows.

use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{AnyElement, App, ClipboardEntry, Context, EntityInputHandler as _, Window, div};
use gpui_base::input::{Enter, Escape, IndentInline, InputEvent, MoveDown, MoveUp, Paste};

use super::{Kind, TextField};

/// A key a composer's screen may take before its text does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposerKey {
    /// Up (the text's `MoveUp`).
    Up,
    /// Down (`MoveDown`).
    Down,
    /// Enter alone (not Shift+Enter, not Mod+Enter).
    Enter,
    /// Tab alone (the text's `IndentInline`).
    Tab,
    /// Escape.
    Escape,
}

/// What the screen did with a key or a paste.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAnswer {
    /// The screen used it; the text does not see it.
    Taken,
    /// The text handles it as it always does.
    Pass,
}

/// One thing on the clipboard a composer can attach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pasted {
    /// An image, as its bytes in the format its MIME type names.
    Image {
        /// Its MIME type (`image/png`, `image/jpeg`, …).
        mime: &'static str,
        /// The encoded image.
        bytes: Vec<u8>,
    },
    /// Files copied in a file manager.
    Files(Vec<PathBuf>),
}

impl TextField {
    /// The caret, as a byte offset into [`Self::text`]: the end of the
    /// selection, and inside marked text (an input method composing) the
    /// marked text's end.
    #[must_use]
    pub fn caret(&self, cx: &App) -> usize {
        match &self.kind {
            Kind::Line(state) => state.read(cx).cursor(),
            Kind::Lines(state, _) => state.read(cx).cursor(),
        }
    }

    /// The selection, as byte offsets into [`Self::text`]; empty (start
    /// equal to end) when nothing is selected (Paseo's recall acts only
    /// then).
    #[must_use]
    pub fn selection(&self, cx: &App) -> Range<usize> {
        match &self.kind {
            Kind::Line(state) => state.read(cx).selected_range(),
            Kind::Lines(state, _) => state.read(cx).selected_range(),
        }
    }

    /// Replace bytes `range` of the text with `with`, the caret after it,
    /// as one edit the person can undo (a menu's choice written over the
    /// word being typed). A range past the text or inside a character is
    /// brought to the nearest character boundary below it.
    pub fn replace_range(
        &self,
        range: Range<usize>,
        with: &str,
        window: &mut Window,
        cx: &mut App,
    ) {
        let text = self.text(cx);
        let units = utf16_range(&text, range);
        match &self.kind {
            Kind::Line(state) => state.update(cx, |state, cx| {
                state.replace_text_in_range(Some(units), with, window, cx);
            }),
            Kind::Lines(state, _) => state.update(cx, |state, cx| {
                state.replace_text_in_range(Some(units), with, window, cx);
            }),
        }
    }

    /// Replace the whole text with `with`, the caret at its end, as one
    /// edit the person can undo (a recalled message, a kept draft).
    /// [`Self::set_text`] instead puts the caret at the start and forgets
    /// the undo history.
    pub fn replace_all(&self, with: &str, window: &mut Window, cx: &mut App) {
        let end = self.text(cx).len();
        self.replace_range(0..end, with, window, cx);
    }
}

impl TextField {
    /// Call `on_edit` on the field's owner after every edit typed into it
    /// (the composer keeps its draft as it is typed). Text the screen writes
    /// (`set_text`, `clear`) is not heard: gpui-base's `set_value` turns its
    /// events off, so an owner that writes the text follows it itself. It
    /// runs beside the field's own handlers, which stay as they are.
    pub fn on_edit<V: 'static>(
        &self,
        on_edit: fn(&mut V, &mut Window, &mut Context<'_, V>),
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) {
        let edited = move |owner: &mut V,
                           event: &InputEvent,
                           window: &mut Window,
                           cx: &mut Context<'_, V>| {
            if matches!(event, InputEvent::Change) {
                on_edit(owner, window, cx);
            }
        };
        match &self.kind {
            Kind::Line(state) => cx
                .subscribe_in(state, window, move |owner, _state, event, window, cx| {
                    edited(owner, event, window, cx);
                })
                .detach(),
            Kind::Lines(state, _) => cx
                .subscribe_in(state, window, move |owner, _state, event, window, cx| {
                    edited(owner, event, window, cx);
                })
                .detach(),
        }
    }
}

/// `range` (bytes of `text`) in UTF-16 units, as the engine's input
/// handler counts; each end brought down to a character boundary.
fn utf16_range(text: &str, range: Range<usize>) -> Range<usize> {
    let floor = |at: usize| {
        let mut at = at.min(text.len());
        while !text.is_char_boundary(at) {
            at = at.saturating_sub(1);
        }
        at
    };
    let start = floor(range.start);
    let end = floor(range.end).max(start);
    let units = |part: &str| part.encode_utf16().count();
    let before = text.get(..start).map_or(0, units);
    let inside = text.get(start..end).map_or(0, units);
    before..before.saturating_add(inside)
}

/// `element` (the composer's text, [`TextField::element`]) with its keys
/// offered to `keys` and its pastes of images or files to `paste` before
/// the text sees them. See the module's decision log.
pub fn hooked(
    field: &TextField,
    element: AnyElement,
    keys: impl Fn(ComposerKey, &mut Window, &mut App) -> KeyAnswer + 'static,
    paste: impl Fn(Vec<Pasted>, &mut Window, &mut App) -> KeyAnswer + 'static,
) -> AnyElement {
    let keys = Rc::new(keys);
    let offer = |key: ComposerKey| {
        let keys = Rc::clone(&keys);
        let field = field.clone();
        move |window: &mut Window, cx: &mut App| {
            // An input method composing owns the key (a Hangul syllable's
            // Enter commits it; its arrows move inside its candidates).
            if field.composing(window, cx) {
                return;
            }
            if keys(key, window, cx) == KeyAnswer::Taken {
                cx.stop_propagation();
            }
        }
    };
    let (up, down, enter, tab, escape) = (
        offer(ComposerKey::Up),
        offer(ComposerKey::Down),
        offer(ComposerKey::Enter),
        offer(ComposerKey::Tab),
        offer(ComposerKey::Escape),
    );
    let field_for_paste = field.clone();
    div()
        .w_full()
        .capture_action(move |_: &MoveUp, window, cx| up(window, cx))
        .capture_action(move |_: &MoveDown, window, cx| down(window, cx))
        .capture_action(move |action: &Enter, window, cx| {
            // Shift+Enter starts a line and Mod+Enter sends, as before.
            if !action.secondary && !action.shift {
                enter(window, cx);
            }
        })
        .capture_action(move |_: &IndentInline, window, cx| tab(window, cx))
        .capture_action(move |_: &Escape, window, cx| escape(window, cx))
        .capture_action(move |_: &Paste, window, cx| {
            if field_for_paste.composing(window, cx) {
                return;
            }
            let pasted = clipboard_attachments(cx);
            if !pasted.is_empty() && paste(pasted, window, cx) == KeyAnswer::Taken {
                cx.stop_propagation();
            }
        })
        .child(element)
        .into_any_element()
}

/// The images and files on the clipboard, in its order; nothing for text.
fn clipboard_attachments(cx: &App) -> Vec<Pasted> {
    let Some(item) = cx.read_from_clipboard() else {
        return Vec::new();
    };
    item.entries()
        .iter()
        .filter_map(|entry| match entry {
            ClipboardEntry::Image(image) => Some(Pasted::Image {
                mime: image.format.mime_type(),
                bytes: image.bytes.clone(),
            }),
            ClipboardEntry::ExternalPaths(paths) => Some(Pasted::Files(paths.paths().to_vec())),
            ClipboardEntry::String(_) => None,
        })
        .collect()
}

#[cfg(test)]
mod tests;
