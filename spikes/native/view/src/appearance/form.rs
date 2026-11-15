//! Pure field transitions over the existing editor's text. The native text
//! engine alone owns caret/IME/history; this layer returns filtered text,
//! parsed preview deltas and one optional saved-preference change.
use super::{DraftText, FontFamily, Number};
use crate::prefs::{Appearance, PreferenceChange};
use std::fmt::Display;

/// The source's Appearance fields, independent of control identity or layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// Interface family; no live preview before commit in the source.
    InterfaceFamily,
    /// Interface size; no local preview override in the source.
    InterfaceSize,
    /// Independent content size with local preview.
    ContentSize,
    /// Code family with local preview.
    MonoFamily,
    /// Independent code size with local preview.
    CodeSize,
    /// Numeric content width, committed on blur/submit.
    Width,
}
impl Field {
    /// Source row order, excluding its immediate syntax selector.
    pub const ALL: [Self; 6] = [
        Self::InterfaceFamily,
        Self::InterfaceSize,
        Self::ContentSize,
        Self::MonoFamily,
        Self::CodeSize,
        Self::Width,
    ];
}

/// Text to restore after commit or an external saved change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldText {
    /// Numeric or family data, never a translated sentence.
    Value(DraftText),
    /// Retained native legacy Full width; the caller supplies its translated name.
    WindowWidth,
}

/// One local preview axis changes; unrelated typed drafts stay untouched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewDelta {
    /// No preview axis corresponds to this field.
    Unchanged,
    /// None restores the saved content size.
    Content(Option<Number>),
    /// None restores the saved code size.
    Code(Option<Number>),
    /// None restores the saved family; an empty valid family means default.
    Mono(Option<FontFamily>),
}

/// The editor applies the filtered text only if it differs from its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edited {
    /// Text retained by the existing engine.
    pub text: DraftText,
    /// Local preview data, never a global preference update.
    pub preview: PreviewDelta,
}

/// Blur/submit restores normalized text and at most one changed preference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Committed {
    /// The value the input should show after committing.
    pub text: FieldText,
    /// None for invalid or unchanged input; other preferences are never replaced.
    pub change: Option<PreferenceChange>,
}

/// Mirror source filtering: size axes use ASCII digits, width/family retain
/// raw spelling. Width is intentionally not filtered by the source input.
#[must_use]
pub fn edit(field: Field, raw: &str) -> Edited {
    let text = match field {
        Field::InterfaceSize | Field::ContentSize | Field::CodeSize => {
            raw.chars().filter(char::is_ascii_digit).collect()
        }
        Field::InterfaceFamily | Field::MonoFamily | Field::Width => raw.to_owned(),
    };
    let preview = match field {
        Field::ContentSize => PreviewDelta::Content(Number::parse(&text)),
        Field::CodeSize => PreviewDelta::Code(Number::parse(&text)),
        Field::MonoFamily => PreviewDelta::Mono(FontFamily::parse(&text).ok()),
        Field::InterfaceFamily | Field::InterfaceSize | Field::Width => PreviewDelta::Unchanged,
    };
    Edited {
        text: DraftText::new(text),
        preview,
    }
}

/// Resolve against the CURRENT saved state, including changes from elsewhere.
#[must_use]
pub fn commit(field: Field, raw: &str, saved: &Appearance) -> Committed {
    match field {
        Field::InterfaceFamily => resolved(
            &saved.interface_family,
            FontFamily::parse(raw).ok(),
            PreferenceChange::InterfaceFamily,
        ),
        Field::MonoFamily => resolved(
            &saved.mono_family,
            FontFamily::parse(raw).ok(),
            PreferenceChange::MonoFamily,
        ),
        Field::InterfaceSize => resolved(
            &saved.interface_size,
            Number::parse(raw).and_then(Number::interface),
            PreferenceChange::InterfaceSize,
        ),
        Field::ContentSize => resolved(
            &saved.content_size,
            Number::parse(raw).and_then(Number::content),
            PreferenceChange::ContentSize,
        ),
        Field::CodeSize => resolved(
            &saved.code_size,
            Number::parse(raw).and_then(Number::code),
            PreferenceChange::CodeSize,
        ),
        Field::Width => {
            let before = saved.content_width;
            let next = Number::parse(raw)
                .and_then(Number::width)
                .map_or(before, |value| before.commit(value));
            Committed {
                text: width_text(next),
                change: (next != before).then_some(PreferenceChange::ContentWidth(next)),
            }
        }
    }
}

/// Saved field text, computed on hydration/change rather than during paint.
#[must_use]
pub fn saved_text(field: Field, saved: &Appearance) -> FieldText {
    match field {
        Field::InterfaceFamily => value(&saved.interface_family),
        Field::MonoFamily => value(&saved.mono_family),
        Field::InterfaceSize => value(&saved.interface_size),
        Field::ContentSize => value(&saved.content_size),
        Field::CodeSize => value(&saved.code_size),
        Field::Width => width_text(saved.content_width),
    }
}

fn resolved<T: Clone + Eq + Display>(
    before: &T,
    parsed: Option<T>,
    change: fn(T) -> PreferenceChange,
) -> Committed {
    let next = parsed.unwrap_or_else(|| before.clone());
    Committed {
        text: value(&next),
        change: (&next != before).then(|| change(next)),
    }
}
fn value(value: &impl Display) -> FieldText {
    FieldText::Value(DraftText::new(value.to_string()))
}
fn width_text(width: crate::prefs::ContentWidth) -> FieldText {
    match width.points() {
        Some(points) => value(&points),
        None => FieldText::WindowWidth,
    }
}

#[cfg(test)]
mod tests;
