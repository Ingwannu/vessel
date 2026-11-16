//! Patch7 behavior through public editor APIs, in native kit test windows.
//! The vendor is a dependency, so its private unit fixtures are not executable
//! from the native workspace. Keep regressions here without widening its API.
use gpui::prelude::*;
use gpui::{
    Context, Entity, EntityInputHandler as _, HighlightStyle, Render, SharedString, TestAppContext,
    Window, div, point, px,
};
use gpui_base::input::{
    BufferRow, DiagnosticColors, Editor, EditorState, HighlightStyleResolver, InputEditorStyle,
    InputLinePaint, InputLinePresentation,
};
use std::sync::{Arc, Mutex};

#[path = "mapping_viewport_tests.rs"]
mod viewport;

struct Syntax;
impl HighlightStyleResolver for Syntax {
    fn style(&self, name: &str) -> Option<HighlightStyle> {
        (name == "keyword").then(|| HighlightStyle {
            color: Some(gpui::hsla(0.6, 0.5, 0.5, 1.)),
            ..HighlightStyle::default()
        })
    }
}
fn complete_literal() -> InputEditorStyle {
    // Match gpui-component's exhaustive literal: adding a public field must
    // fail here, instead of silently escaping through ..Default.
    InputEditorStyle {
        foreground: gpui::white(),
        muted_foreground: gpui::black(),
        background: gpui::white(),
        border: gpui::black(),
        selection: gpui::hsla(0.6, 0.5, 0.5, 0.4),
        caret: gpui::black(),
        diagnostics: DiagnosticColors::default(),
        highlight_styles: Arc::new(Syntax),
        editor_invisible: None,
        editor_active_line: None,
        editor_gutter_background: None,
        fold_icon_renderer: None,
    }
}
#[derive(Clone)]
struct Mapping {
    sample: SharedString,
    rows: Arc<Mutex<Vec<usize>>>,
    colour: gpui::Hsla,
}
impl InputLinePresentation for Mapping {
    fn gutter_sample(&self) -> SharedString {
        self.sample.clone()
    }
    fn line(&self, row: BufferRow) -> InputLinePaint {
        self.rows.lock().unwrap().push(row.index());
        InputLinePaint {
            number: Some("42".into()),
            number_color: None,
            background: Some(self.colour),
        }
    }
}
impl Mapping {
    fn new(sample: &'static str) -> Self {
        Self {
            sample: sample.into(),
            rows: Arc::default(),
            colour: gpui::hsla(0.23, 0.91, 0.42, 0.31),
        }
    }
    fn style(&self) -> InputEditorStyle {
        complete_literal().with_line_presentation(Some(Arc::new(self.clone())))
    }
}
#[test]
fn complete_style_literals_and_nested_clear_preserve_syntax() {
    let original = complete_literal();
    assert!(original.line_presentation().is_none());
    assert!(InputEditorStyle::default().line_presentation().is_none());
    let mapped = original
        .clone()
        .with_line_presentation(Some(Arc::new(Mapping::new("11"))))
        .with_line_presentation(Some(Arc::new(Mapping::new("2222"))));
    assert_eq!(
        mapped.line_presentation().unwrap().gutter_sample().as_ref(),
        "2222"
    );
    for name in ["keyword", "other"] {
        assert_eq!(
            mapped.highlight_styles.style(name).map(|s| s.color),
            original.highlight_styles.style(name).map(|s| s.color)
        );
    }
    let cleared = mapped.with_line_presentation(None);
    assert!(cleared.line_presentation().is_none());
    assert_eq!(cleared.foreground, original.foreground);
    for name in ["keyword", "other"] {
        assert_eq!(
            cleared.highlight_styles.style(name).map(|s| s.color),
            original.highlight_styles.style(name).map(|s| s.color)
        );
    }
}
struct Probe {
    state: Entity<EditorState>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        div().size_full().child(Editor::new(&self.state))
    }
}
fn input(
    text: &str,
    style: InputEditorStyle,
    window: &mut Window,
    cx: &mut Context<'_, Probe>,
) -> Probe {
    Probe {
        state: cx.new(|cx| {
            let mut state = EditorState::new(window, cx)
                .soft_wrap(false)
                .folding(false)
                .default_value(text);
            state.set_readonly(true, cx);
            state.set_scroll_beyond_last_line(Some(0), window, cx);
            state.set_editor_style(style);
            state
        }),
    }
}
#[gpui::test]
fn none_retains_gutter_and_mapped_sample_changes_actual_text_origin(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    let (view, cx) = cx.add_window_view(|w, cx| input("one\ntwo", complete_literal(), w, cx));
    let state = view.read_with(cx, |view, _| view.state.clone());
    let caret = |cx: &mut gpui::VisualTestContext| {
        cx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
            state.update(cx, |state, cx| {
                state
                    .bounds_for_range(0..0, state.input_bounds(), window, cx)
                    .unwrap()
            })
        })
    };
    let before = caret(cx);
    cx.update(|_, cx| {
        state.update(cx, |state, _| {
            state.set_editor_style(complete_literal().with_line_presentation(None));
        });
    });
    let cleared = caret(cx);
    assert_eq!(before, cleared);
    let small = Mapping::new("99");
    cx.update(|_, cx| state.update(cx, |state, _| state.set_editor_style(small.style())));
    let narrow = caret(cx);
    let wide = Mapping::new("9999999");
    cx.update(|_, cx| state.update(cx, |state, _| state.set_editor_style(wide.style())));
    let large = caret(cx);
    assert!(
        large.left() > narrow.left(),
        "mapped font width changes the real gutter"
    );
    assert_eq!(large.top(), narrow.top());
    let rows = wide.rows.lock().unwrap();
    assert_eq!(
        &*rows,
        &[0, 1],
        "one projection per visible buffer row in this frame"
    );
}
#[gpui::test]
fn mapped_tint_keeps_bounds_wrap_height_and_readonly_selection_copy(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    let mapping = Mapping::new("99999");
    let text = format!("{}\n새 줄\nlast", "wide ".repeat(80));
    let (view, cx) = cx.add_window_view(|w, cx| input(&text, mapping.style(), w, cx));
    cx.update(|window, cx| {
        window.activate_window();
        view.update(cx, |view, cx| {
            view.state.update(cx, |state, cx| {
                state.set_soft_wrap(true, window, cx);
                state.focus(window, cx);
                state.select_all(window, cx);
            });
        });
        window.draw(cx).clear(cx);
        let quads = window.painted_quads();
        let tints = quads
            .iter()
            .filter(|q| q.background == gpui::solid_background(mapping.colour))
            .collect::<Vec<_>>();
        assert!(
            tints.len() >= 2,
            "body and fixed gutter both receive the mapping"
        );
        let line_height = view
            .read(cx)
            .state
            .read(cx)
            .line_height()
            .unwrap()
            .scale(window.scale_factor());
        assert!(
            tints.iter().any(|q| q.bounds.size.height > line_height),
            "the source-row tint covers its wrapped continuation rows"
        );
        let viewport = window.viewport_size().width.scale(window.scale_factor());
        assert!(
            tints
                .iter()
                .all(|q| within_viewport(q.bounds.right(), viewport))
        );
        view.update(cx, |view, cx| {
            view.state.update(cx, |state, cx| {
                assert!(state.display_map().wrap_row_count() > 3);
                assert_eq!(state.selected_text().to_string(), text);
                state.replace_text_in_range(None, "blocked", window, cx);
                assert_eq!(state.value(), text);
            });
        });
    });
    cx.simulate_keystrokes(if cfg!(target_os = "macos") {
        "cmd-c"
    } else {
        "ctrl-c"
    });
    cx.run_until_parked();
    cx.update(|_, cx| {
        assert_eq!(cx.read_from_clipboard().and_then(|v| v.text()), Some(text));
    });
}

// Pixel operators have no checked counterpart. Keep the expected finite test
// geometry in these smallest helpers; no count/offset arithmetic is unchecked.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "finite test-only pixel extent from a known row count and the test window font"
)]
fn extent(rows: usize, line: gpui::Pixels) -> gpui::Pixels {
    line * rows
}
#[expect(
    clippy::arithmetic_side_effects,
    reason = "finite test-only distance between two neighboring shaped rows"
)]
fn row_distance(first: gpui::Pixels, next: gpui::Pixels) -> gpui::Pixels {
    next - first
}
#[expect(
    clippy::arithmetic_side_effects,
    reason = "finite physical viewport with the existing one-pixel paint rounding tolerance"
)]
fn within_viewport(right: gpui::ScaledPixels, viewport: gpui::ScaledPixels) -> bool {
    right <= viewport + gpui::ScaledPixels(1.)
}
