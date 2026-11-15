//! Literal source feedback shape, with no UI/OS raster claim.
use super::*;
#[test]
fn model_feedback_uses_source_padding_gap_and_muted_base_type_in_both_themes() {
    for scheme in [crate::Scheme::Light, crate::Scheme::Dark] {
        let tokens = Tokens::of(scheme);
        let mut body = frame(&tokens);
        let style = body.style();
        assert_eq!(style.padding.left, Some(px(16.0).into()));
        assert_eq!(style.padding.top, Some(px(16.0).into()));
        assert_eq!(style.gap.height, Some(px(8.0).into()));
        assert_eq!(style.text.font_size, Some(px(14.0).into()));
        assert_eq!(style.text.color, Some(tokens.neutral.foreground_muted));
        assert_eq!(style.border_widths.left, None);
        assert_eq!(style.background, None);
    }
}
