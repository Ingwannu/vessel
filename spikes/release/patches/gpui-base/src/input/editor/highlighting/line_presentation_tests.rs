// MODIFIED by Vessel (Team-WICKED), 2026-11-15: style literal compatibility (spikes/release/patches/README.md, "gpui-base", change 7)
// Vessel patch: an external exhaustive literal is a compatibility obligation.
use super::*;
use std::sync::Arc;

struct Syntax;
impl HighlightStyleResolver for Syntax {
    fn style(&self, name: &str) -> Option<HighlightStyle> {
        (name == "keyword").then(|| HighlightStyle {
            color: Some(gpui::hsla(0.6, 0.5, 0.5, 1.)),
            ..HighlightStyle::default()
        })
    }
}
struct Lines(&'static str);
impl InputLinePresentation for Lines {
    fn gutter_sample(&self) -> SharedString {
        self.0.into()
    }
    fn line(&self, _row: BufferRow) -> InputLinePaint {
        InputLinePaint {
            number: Some(self.0.into()),
            ..InputLinePaint::default()
        }
    }
}
fn external_literal() -> InputEditorStyle {
    // The same complete field set gpui-component0.8.1 supplies. No ..Default
    // escape: another public field must fail compilation of this regression.
    InputEditorStyle {
        foreground: gpui::white(),
        muted_foreground: gpui::black(),
        background: gpui::white(),
        border: gpui::black(),
        selection: gpui::black(),
        caret: gpui::black(),
        diagnostics: DiagnosticColors::default(),
        highlight_styles: Arc::new(Syntax),
        editor_invisible: None,
        editor_active_line: None,
        editor_gutter_background: None,
        fold_icon_renderer: None,
    }
}
#[test]
fn old_complete_literals_and_default_resolvers_have_no_line_mapping() {
    let style = external_literal();
    assert!(style.line_presentation().is_none());
    assert!(InputEditorStyle::default().line_presentation().is_none());
    assert_eq!(
        style.highlight_styles.style("keyword").unwrap().color,
        Some(gpui::hsla(0.6, 0.5, 0.5, 1.))
    );
    assert!(style.highlight_styles.style("other").is_none());
}
#[test]
fn nested_outer_mapping_wins_and_clear_preserves_every_syntax_lookup() {
    let original = external_literal();
    let style = original
        .clone()
        .with_line_presentation(Some(Arc::new(Lines("11"))))
        .with_line_presentation(Some(Arc::new(Lines("2222"))));
    assert_eq!(
        style
            .line_presentation()
            .unwrap()
            .line(BufferRow::new(0))
            .number,
        Some("2222".into())
    );
    assert_eq!(
        style.line_presentation().unwrap().gutter_sample(),
        SharedString::from("2222")
    );
    for name in ["keyword", "other"] {
        assert_eq!(
            style.highlight_styles.style(name).map(|style| style.color),
            original
                .highlight_styles
                .style(name)
                .map(|style| style.color)
        );
    }
    let cleared = style.with_line_presentation(None);
    assert!(cleared.line_presentation().is_none());
    assert_eq!(cleared.foreground, original.foreground);
    for name in ["keyword", "other"] {
        assert_eq!(
            cleared
                .highlight_styles
                .style(name)
                .map(|style| style.color),
            original
                .highlight_styles
                .style(name)
                .map(|style| style.color)
        );
    }
}
