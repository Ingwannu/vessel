//! One multiline input factory; Plain and Identified share TextareaState/listen.
//!
//! [Decision Log]
//! - Purpose: scheduled prompts need their actual input source, as lines do.
//! - Existing assets: the checked FieldId and Plain/Identified callback enum.
//! - Choice: feed that enum into the SAME multiline factory, preserving all
//!   old engines/Enter/growth/Editability/FormMinimum defaults and signatures.
//! - Why/cost: no second subscription/textarea or visit counter; moving the
//!   constructors and SendOn together keeps registration headroom in the root.
use super::{
    Callbacks, FormMinimum, Handlers, Kind, Known, MultilineStyle, TextField, TextareaState,
    listen, register,
};
use gpui::{Context, Window};
use std::{cell::Cell, rc::Rc};
/// The existing composer's maximum visible complete lines.
const COMPOSER_LINES: usize = 12;
/// The existing form's maximum visible complete lines.
const FORM_LINES: usize = 12;
/// The existing form's minimum complete lines before its outer minimum.
const FORM_MIN_LINES: usize = 3;

/// Which press sends a composer's text (Settings › General › Send
/// shortcut, the page's `compose-keys.ts`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SendOn {
    /// Enter (and Mod+Enter) sends; Shift+Enter starts a line.
    #[default]
    Enter,
    /// Mod+Enter sends; Enter and Shift+Enter start a line.
    ModEnter,
}

impl SendOn {
    /// Whether an Enter that is not Shift+Enter sends; `with_mod` when
    /// Command (Control elsewhere) was held.
    #[must_use]
    pub const fn sends(self, with_mod: bool) -> bool {
        match self {
            Self::Enter => true,
            Self::ModEnter => with_mod,
        }
    }
}

impl TextField {
    /// A composer's text: lines that grow; the press [`SendOn`] names
    /// (Enter unless [`Self::set_send_on`] says otherwise) calls
    /// `on_submit` with the text, Shift+Enter starts a new line.
    ///
    /// The engine sends on Enter and Mod+Enter and starts a line on
    /// Shift+Enter (`submit_on_enter`); in [`SendOn::ModEnter`] a plain
    /// Enter that reaches here starts a line instead (the engine's own
    /// Enter-starts-a-line mode would start one on Mod+Enter too, before
    /// sending).
    pub fn composer<V: 'static>(
        placeholder: &str,
        on_submit: fn(&mut V, String, &mut Window, &mut Context<'_, V>),
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        Self::multiline(
            MultilineStyle::Composer,
            placeholder,
            Callbacks::Plain {
                on_submit,
                on_change: None,
            },
            window,
            cx,
        )
    }
    /// A multiline form field on the one-line form field's surface.
    /// Enter starts a line; Mod+Enter submits. It grows from three to
    /// twelve lines, then the existing text engine scrolls. GOAT's small
    /// textarea supplies the 68 / 80-point minimum and 256-point cap.
    pub fn form_multiline<V: 'static>(
        placeholder: &str,
        on_submit: fn(&mut V, String, &mut Window, &mut Context<'_, V>),
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        Self::multiline(
            MultilineStyle::Form,
            placeholder,
            Callbacks::Plain {
                on_submit,
                on_change: Some(|_, _, cx| cx.notify()),
            },
            window,
            cx,
        )
    }

    pub(super) fn multiline<V: 'static>(
        style: MultilineStyle,
        placeholder: &str,
        callbacks: Callbacks<V>,
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        let placeholder = placeholder.to_owned();
        let minimum = match style {
            MultilineStyle::Composer => 1,
            MultilineStyle::Form => FORM_MIN_LINES,
        };
        let maximum = match style {
            MultilineStyle::Composer => COMPOSER_LINES,
            MultilineStyle::Form => FORM_LINES,
        };
        let state = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(minimum, maximum)
                .submit_on_enter(true)
                .placeholder(placeholder)
        });
        let focused = Rc::new(Cell::new(false));
        let shortcut = match style {
            MultilineStyle::Composer => SendOn::Enter,
            MultilineStyle::Form => SendOn::ModEnter,
        };
        let send = Rc::new(Cell::new(shortcut));
        listen(
            &state,
            Handlers {
                callbacks,
                focused: Rc::clone(&focused),
                send: Some(Rc::clone(&send)),
            },
            window,
            cx,
        );
        register(Known::Lines(state.downgrade()));
        Self {
            kind: Kind::Lines(state, style),
            focused,
            form_minimum: FormMinimum::Default,
            send,
        }
    }

    /// Source-identified form textarea using the existing engine and minimum.
    /// The callback source is editor identity, not a form visit or authority.
    #[must_use]
    pub fn form_multiline_with_source<V: 'static>(
        placeholder: &str,
        on_submit: fn(&mut V, super::FieldId, String, &mut Window, &mut Context<'_, V>),
        on_change: fn(&mut V, super::FieldId, String, &mut Context<'_, V>),
        minimum: FormMinimum,
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        let mut field = Self::multiline(
            MultilineStyle::Form,
            placeholder,
            Callbacks::Identified {
                on_submit,
                on_change,
            },
            window,
            cx,
        );
        field.form_minimum = minimum;
        field
    }
}

#[cfg(test)]
mod tests;
