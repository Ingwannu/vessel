//! Built-in and contributed paints share one actual global/component projection.
use super::*;
use crate::theme::palette::ContributedColors;

#[gpui::test]
fn custom_same_scheme_replacement_updates_component_colors_without_replacing_syntax(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(|cx| {
        init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap();
        let before_syntax = syntax::selected();
        let before_content = fonts::content_size();
        let before_code = fonts::code_size();
        let named = Tokens::palette(Palette::Zinc);
        let colors = ContributedColors {
            background: gpui::rgba(0x1020_30a0).into(),
            foreground: named.neutral.foreground,
            raised: named.neutral.surface1,
            control: named.neutral.surface2,
            border: named.neutral.border,
            accent: named.neutral.accent,
            muted_foreground: named.neutral.foreground_muted,
            ring: named.neutral.foreground_extra_muted,
        };
        let custom = Tokens::contributed(Scheme::Dark, colors);
        set_tokens(cx, &custom);
        assert_eq!(tokens(cx), custom);
        assert_eq!(
            gpui_component::Theme::global(cx).colors.background,
            colors.background
        );
        assert_eq!(
            gpui_component::Theme::global(cx).colors.foreground,
            colors.foreground
        );
        assert_eq!(syntax::selected(), before_syntax);
        assert_eq!(fonts::content_size().to_bits(), before_content.to_bits());
        assert_eq!(fonts::code_size().to_bits(), before_code.to_bits());
        set_palette(cx, Palette::Midnight);
        assert_eq!(tokens(cx), Tokens::palette(Palette::Midnight));
        assert_eq!(
            gpui_component::Theme::global(cx).colors.background,
            tokens(cx).bg_base
        );
    });
}
