//! The page's `file-panels.test.ts` cases.

use super::*;

#[test]
fn the_extension_is_after_the_base_names_last_dot_in_lower_case() {
    assert_eq!(extension("docs/README.MD"), "md");
    assert_eq!(extension("a.b/notes"), "");
    assert_eq!(extension("dir\\photo.Jpeg"), "jpeg");
    assert_eq!(extension(".md"), "md");
}

#[test]
fn markdown_html_and_images_open_in_their_panels_the_rest_as_source() {
    assert_eq!(panel_of("docs/plan.markdown"), FilePanel::Markdown);
    assert_eq!(panel_of("site/index.htm"), FilePanel::Html);
    for name in ["a.png", "a.jpg", "a.jpeg", "a.gif", "a.webp", "a.svg"] {
        assert!(matches!(panel_of(name), FilePanel::Image(_)), "{name}");
    }
    assert_eq!(panel_of("src/main.rs"), FilePanel::Source);
    assert_eq!(panel_of("Makefile"), FilePanel::Source);
    assert_eq!(image_kind("logo.SVG"), Some(ImageKind::Svg));
    assert_eq!(image_kind("photo.jpg"), Some(ImageKind::Jpeg));
    assert_eq!(image_kind("main.rs"), None);
}

#[test]
fn only_markdown_has_a_preview_in_the_app_and_html_stays_source() {
    assert!(FilePanel::Markdown.has_preview());
    assert!(!FilePanel::Html.has_preview());
    assert!(!FilePanel::Image(ImageKind::Png).has_preview());
    assert!(!FilePanel::Source.has_preview());
}

#[test]
fn a_file_opens_on_preview_unless_opened_at_a_line() {
    assert_eq!(Side::opening(false), Side::Preview);
    assert_eq!(Side::opening(true), Side::Source);
}

#[test]
fn a_preview_is_drawn_up_to_paseos_native_display_budget() {
    assert!(preview_fits(PREVIEW_LIMIT));
    assert!(!preview_fits(PREVIEW_LIMIT.saturating_add(1)));
}
