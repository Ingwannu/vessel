//! Why: Paseo shows an immutable form value without the select affordances.
//! Same frame/text path, no anchor, input engine, hover, disabled fade or action.
use super::style::{FramePresentation, geometry, paint};
use super::{frame, label_frame};
use crate::{ControlSizing, Tokens};
use gpui::prelude::*;
use gpui::{Div, SharedString};
/// Pinned schedule-form-sheet.tsx889–919/1042–1058 readonly target field.
#[must_use]
pub fn readonly_value(
    label: impl Into<SharedString>,
    sizing: ControlSizing,
    tokens: &Tokens,
) -> Div {
    frame(
        geometry(FramePresentation::ReadOnlyForm, sizing),
        paint(FramePresentation::ReadOnlyForm, tokens),
    )
    .w_full()
    .min_w_0()
    .child(label_frame().flex_1().child(label.into()))
}
#[cfg(test)]
mod tests {
    use super::super::style::Radius;
    use super::*;
    use crate::{Scheme, Type};
    use gpui::px;
    #[test]
    fn readonly_form_has_literal_source_values_in_both_themes_and_layouts() {
        for (scheme, surface, border, foreground) in [
            (Scheme::Light, 0x00f4_f4f5, 0x00e4_e4e7, 0x001a_1a1e),
            (Scheme::Dark, 0x0027_2a29, 0x0025_2b2a, 0x00fa_fafa),
        ] {
            let tokens = Tokens::of(scheme);
            let paint = paint(FramePresentation::ReadOnlyForm, &tokens);
            assert_eq!(paint.background, gpui::rgb(surface).into());
            assert_eq!(paint.border, gpui::rgb(border).into());
            assert_eq!(paint.foreground, gpui::rgb(foreground).into());
            assert_eq!(paint.hover, None);
            assert_eq!(paint.pressed, None);
            for (sizing, height, vertical) in [
                (ControlSizing::Pointer, 32.0, 5.0),
                (ControlSizing::Touch, 44.0, 11.0),
            ] {
                let g = geometry(FramePresentation::ReadOnlyForm, sizing);
                assert_eq!(g.minimum, Some(height));
                assert_eq!(g.horizontal.to_bits(), 12.0_f32.to_bits());
                assert_eq!(g.vertical, Some(vertical));
                assert_eq!(g.radius, Radius::Points(8.0));
                assert_eq!(g.type_, Type::Body);
                let (font, line, _) = Type::Body.metrics();
                assert_eq!(font.to_bits(), 14.0_f32.to_bits());
                assert_eq!(line.to_bits(), 20.0_f32.to_bits());
                let mut value = readonly_value("An immutable chat", sizing, &tokens);
                assert_eq!(value.style().min_size.height, Some(px(height).into()));
                assert_eq!(value.style().padding.left, Some(px(12.0).into()));
                assert_eq!(value.style().border_widths.left, Some(px(1.0).into()));
                assert_eq!(value.style().opacity, None);
            }
        }
    }
}
