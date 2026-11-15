//! Parsed preview axes only. Raw drafts remain in the native text engine;
//! each edit is parsed once at the form edge and unrelated axes survive it.
use super::{Field, PreviewDelta};
use crate::{
    appearance::{FontFamily, Number},
    prefs::{Appearance, CodeSize, ContentSize},
};

/// Local parsed overrides. None follows the current saved axis.
#[derive(Debug, Default)]
pub struct PreviewDrafts {
    content: Option<Number>,
    code: Option<Number>,
    mono: Option<FontFamily>,
}
/// Drawable bounded values, resolved independently from the current saved axes.
#[derive(Debug)]
pub struct PreviewValues<'a> {
    /// Absolute content units, never multiplied by UI scale.
    pub content: ContentSize,
    /// Absolute code units, never multiplied by content scale.
    pub code: CodeSize,
    /// Parsed stack spelling and faces, not a path or executable font source.
    pub mono: &'a FontFamily,
}
impl PreviewDrafts {
    /// Apply the already parsed edit, leaving every unrelated axis alone.
    pub fn apply(&mut self, delta: PreviewDelta) {
        match delta {
            PreviewDelta::Unchanged => {}
            PreviewDelta::Content(value) => self.content = value,
            PreviewDelta::Code(value) => self.code = value,
            PreviewDelta::Mono(value) => self.mono = value,
        }
    }
    /// Commit/external resync clears only the corresponding local override.
    pub fn reset(&mut self, field: Field) {
        match field {
            Field::ContentSize => self.content = None,
            Field::CodeSize => self.code = None,
            Field::MonoFamily => self.mono = None,
            Field::InterfaceFamily | Field::InterfaceSize | Field::Width => {}
        }
    }
    /// Native preview bounds equal committed bounds, preventing huge glyph
    /// allocations from uncommitted finite numeric drafts. No parse in paint.
    #[must_use]
    pub fn values<'a>(&'a self, saved: &'a Appearance) -> PreviewValues<'a> {
        PreviewValues {
            content: self
                .content
                .and_then(Number::content)
                .unwrap_or(saved.content_size),
            code: self.code.and_then(Number::code).unwrap_or(saved.code_size),
            mono: self.mono.as_ref().unwrap_or(&saved.mono_family),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parsed_axes_are_independent_and_resync_does_not_erase_other_drafts() {
        let mut drafts = PreviewDrafts::default();
        let mut saved = Appearance::default();
        drafts.apply(super::super::edit(Field::CodeSize, "19").preview);
        drafts.apply(super::super::edit(Field::MonoFamily, "Named, monospace").preview);
        drafts.apply(super::super::edit(Field::InterfaceSize, "21").preview);
        assert_eq!(drafts.values(&saved).code.get(), 19);
        saved.content_size = ContentSize::try_from(18).unwrap();
        drafts.reset(Field::ContentSize);
        assert_eq!(drafts.values(&saved).content.get(), 18);
        assert_eq!(drafts.values(&saved).code.get(), 19);
        assert_eq!(drafts.values(&saved).mono.as_str(), "Named, monospace");
        drafts.apply(super::super::edit(Field::CodeSize, "").preview);
        assert_eq!(drafts.values(&saved).code, saved.code_size);
        drafts.apply(super::super::edit(Field::ContentSize, "999999999").preview);
        assert_eq!(drafts.values(&saved).content.get(), ContentSize::MAX);
        drafts.reset(Field::MonoFamily);
        assert_eq!(drafts.values(&saved).mono, &saved.mono_family);
    }
}
