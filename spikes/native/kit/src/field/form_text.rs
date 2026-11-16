//! Explicit text presentation on the identified form-line factory, same engine.
//!
//! [Decision Log]
//! - Purpose: cadence-editor.tsx106–131 changes only raw cron to `fontFamily.mono`.
//! - Existing assets: form shell14/20, `FieldId`/`listen`, canonical `MonoBody`14/20.
//! - Choice: `Body`/`Code` data on the existing field; only the new identified-line
//!   constructor opts into `Code`. All old factories still choose `Body`, including
//!   `SearchField`, multiline and composer. No area font wrapper or editor exists.
//! - Tradeoff: the canonical code preference scales `Code` exactly as other mono
//!   content. Typography does not grant form-visit/backend authority to `FieldId`.
use super::{FieldId, TextField};
use crate::{Type, TypeExt as _};
use gpui::{Context, Div, Window};
/// Form text face; the shell still owns its dimensions, colours and focus.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FormText {
    /// Existing form `Body`14 with explicit20-point line, unchanged.
    #[default]
    Body,
    /// Canonical `MonoBody`14/20 normal, on the code preference scale.
    Code,
}
impl FormText {
    pub(super) fn apply(self, text: Div) -> Div {
        match self {
            Self::Body => text,
            Self::Code => text.type_(Type::MonoBody),
        }
    }
}
impl TextField {
    /// Identified one-line form with an explicit `Body`/`Code` text presentation.
    /// Same engine, source callbacks and input-method path as `with_source`.
    #[must_use]
    pub fn with_source_text<V: 'static>(
        placeholder: &str,
        on_submit: fn(&mut V, FieldId, String, &mut Window, &mut Context<'_, V>),
        on_change: fn(&mut V, FieldId, String, &mut Context<'_, V>),
        text: FormText,
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        let mut field = Self::with_source(placeholder, on_submit, on_change, window, cx);
        field.form_text = text;
        field
    }
}
#[cfg(test)]
mod tests;
