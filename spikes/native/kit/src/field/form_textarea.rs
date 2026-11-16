//! Source MD textarea chrome on the same form engine; compact defaults stay.
//! Why: changing only the minimum cannot turn GOAT's px9/py3 into the source
//! schedule form's ordinary MD px12/py5-or11. The area must not paint that shell.
use super::{FORM_INSET_X, FORM_INSET_Y, FORM_MAX, FieldId, FormMinimum, TextField};
use crate::{ControlSizing, Tokens, control_sizing::md_field_vertical};
use gpui::prelude::*;
use gpui::{Context, Div, Window, px};
/// Form textarea chrome, independent of the chosen minimum and text engine.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FormTextareaPresentation {
    /// Existing GOAT compact9/3 insets and all old constructor defaults.
    #[default]
    Compact,
    /// Source ordinary MD12/5 pointer or12/11 touch.
    Md,
}
impl FormTextareaPresentation {
    pub(super) fn frame(
        self,
        focused: bool,
        tokens: &Tokens,
        narrow: bool,
        minimum: FormMinimum,
    ) -> Div {
        let frame = super::form_shell(focused, tokens, narrow)
            .flex_col()
            .items_stretch()
            .min_h(px(minimum.height(narrow)))
            .max_h(px(FORM_MAX));
        match self {
            Self::Compact => frame.px(px(FORM_INSET_X)).py(px(FORM_INSET_Y)),
            Self::Md => frame.py(px(md_field_vertical(ControlSizing::from_compact(narrow)))),
        }
    }
}
impl TextField {
    /// Identified form textarea with explicit compact/MD chrome, same engine.
    /// The minimum, source callbacks, Enter/editability and cap are unchanged.
    #[must_use]
    pub fn form_multiline_with_source_presentation<V: 'static>(
        placeholder: &str,
        on_submit: fn(&mut V, FieldId, String, &mut Window, &mut Context<'_, V>),
        on_change: fn(&mut V, FieldId, String, &mut Context<'_, V>),
        presentation: FormTextareaPresentation,
        minimum: FormMinimum,
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        let mut field = Self::form_multiline_with_source(
            placeholder,
            on_submit,
            on_change,
            minimum,
            window,
            cx,
        );
        field.form_textarea = presentation;
        field
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_md_insets_do_not_change_compact_defaults_or_minimum_semantics() {
        assert_eq!(
            FormTextareaPresentation::default(),
            FormTextareaPresentation::Compact
        );
        for scheme in [crate::Scheme::Light, crate::Scheme::Dark] {
            let tokens = Tokens::of(scheme);
            for (narrow, source_y, legacy_height) in [(false, 5.0, 68.0), (true, 11.0, 80.0)] {
                for minimum in [FormMinimum::Default, FormMinimum::Expanded] {
                    let height = match minimum {
                        FormMinimum::Default => legacy_height,
                        FormMinimum::Expanded => 96.0,
                    };
                    for (presentation, x, y) in [
                        (FormTextareaPresentation::Compact, 9.0, 3.0),
                        (FormTextareaPresentation::Md, 12.0, source_y),
                    ] {
                        let mut frame = presentation.frame(false, &tokens, narrow, minimum);
                        let style = frame.style();
                        assert_eq!(style.padding.left, Some(px(x).into()));
                        assert_eq!(style.padding.top, Some(px(y).into()));
                        assert_eq!(style.min_size.height, Some(px(height).into()));
                        assert_eq!(style.max_size.height, Some(px(256.0).into()));
                        assert_eq!(style.background, Some(tokens.neutral.surface2.into()));
                        assert_eq!(style.text.font_size, Some(px(14.0).into()));
                        assert_eq!(style.text.line_height, Some(px(20.0).into()));
                    }
                }
            }
        }
    }
}
