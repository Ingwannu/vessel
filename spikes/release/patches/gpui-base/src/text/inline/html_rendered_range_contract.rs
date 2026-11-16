//! Failing-first behavior over the existing engine, before opt-in/ink code.
use super::*;
use crate::text::{RangeHighlight, RangeHighlightError, SelectionFormat, TextView, TextViewState};
use gpui::{
    AppContext as _, Context, Entity, ParentElement as _, Render, Styled as _, TestAppContext,
};

// Implementation later adds only the explicit range opt-in here. Baseline must
// reach the real Unsupported result, rather than fail on a nonexistent API.
fn literal_state(text: &str, cx: &mut Context<TextViewState>) -> TextViewState {
    TextViewState::html(&literal_html(text), cx)
        .rendered_range_support(crate::text::RenderedRangeSupport::HtmlAndMarkdown)
}

fn literal_html(text: &str) -> String {
    let mut escaped = String::from("<pre>\n");
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            other => escaped.push(other),
        }
    }
    escaped.push_str("</pre>");
    escaped
}

fn html(text: &str, cx: &mut TestAppContext) -> Entity<TextViewState> {
    cx.update(crate::init);
    let state = cx.update(|cx| cx.new(|cx| literal_state(text, cx)));
    cx.run_until_parked();
    state
}

fn highlight(range: Range<usize>) -> RangeHighlight {
    RangeHighlight::new(range, gpui::hsla(0.12, 0.8, 0.6, 0.8))
}

#[gpui::test]
fn html_rendered_range_contract_literal_utf8_and_plain_copy_are_addressable(
    cx: &mut TestAppContext,
) {
    let text = "\n  **literal** <>& \"quotes\" 한글🙂 e\u{301}\n\t two  spaces  ";
    let state = html(text, cx);
    state.update(cx, |state, cx| {
        let expected = format!("{text}\n");
        assert_eq!(state.rendered_text().as_str(), expected);
        state.set_selection_format(SelectionFormat::Plain, cx);
        state.select_all(cx);
        assert_eq!(state.selected_text(), expected);
        let rendered = state.rendered_text();
        let start = rendered.as_str().find("한글🙂").unwrap();
        assert_eq!(
            state.set_range_highlights([highlight(start..start + "한글🙂".len())], cx),
            Ok(()),
            "the opted-in literal body must actually accept the current rendered range"
        );
        assert_eq!(
            state.reveal_range(start..start + "한글🙂".len(), cx),
            Ok(()),
            "range acceptance alone must not hide an Unsupported reveal"
        );
        assert_eq!(state.selected_text(), expected);
        assert_eq!(
            state.set_range_highlights([highlight(start + 1..start + 3)], cx),
            Err(RangeHighlightError::InvalidRange(0)),
            "an interior UTF-8 boundary is refused rather than interpreted as source"
        );
        assert_eq!(
            state.set_range_highlights(Vec::<RangeHighlight>::new(), cx),
            Ok(())
        );
        assert_eq!(state.selected_text(), expected);
    });
}

#[gpui::test]
fn html_rendered_range_contract_default_html_remains_unsupported(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let state = cx.update(|cx| cx.new(|cx| TextViewState::html("<p>default</p>", cx)));
    cx.run_until_parked();
    state.update(cx, |state, cx| {
        assert_eq!(
            state.set_range_highlights([highlight(0..3)], cx),
            Err(RangeHighlightError::Unsupported)
        );
        assert_eq!(
            state.reveal_range(0..3, cx),
            Err(RangeHighlightError::Unsupported)
        );
    });
}

struct Root {
    state: Entity<TextViewState>,
}
impl Render for Root {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui::div()
            .w(px(320.))
            .child(TextView::new(&self.state).selectable(true))
    }
}

fn painted_runs(text: &str) -> Vec<TextRun> {
    RETAINED_LAYOUTS.with(|layouts| {
        layouts
            .borrow()
            .values()
            .find(|layout| layout.text.as_ref() == text && layout.state.upgrade().is_some())
            .expect("the same drawn TextView must have a retained layout")
            .runs
            .clone()
    })
}

// Implementation later adds only the optional foreground builder here. The
// baseline still draws a valid background range so a real ink assertion fails.
fn current_highlight(range: Range<usize>, background: Hsla, foreground: Hsla) -> RangeHighlight {
    RangeHighlight::new(range, background).with_foreground(Some(foreground))
}

#[gpui::test]
fn html_rendered_range_contract_current_ink_reaches_actual_retained_runs(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let text = "current ink baseline suffix";
    let (root, cx) = cx.add_window_view(|_, cx| Root {
        state: cx.new(|cx| TextViewState::markdown(text, cx)),
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let before = painted_runs(text);
    let state = root.read_with(cx, |root, _| root.state.clone());
    let ink: Hsla = gpui::rgb(0x241304).into();
    let fill: Hsla = gpui::rgb(0xfbbf24).into();
    state.update(cx, |state, cx| {
        assert_eq!(
            state.set_range_highlights([current_highlight(0..7, fill, ink)], cx),
            Ok(())
        );
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let after = painted_runs(text);
    let mut at = 0;
    for run in &after {
        assert_eq!(
            run.font, before[0].font,
            "foreground does not replace the font"
        );
        if at < 7 {
            assert_eq!(
                run.color, ink,
                "current-match ink must reach the actual drawn run"
            );
        } else {
            assert_eq!(run.color, before[0].color, "outside ink remains unchanged");
        }
        at += run.len;
    }
    assert_eq!(at, text.len());
    state.update(cx, |state, cx| state.clear_range_highlights(cx));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(
        painted_runs(text),
        before,
        "clear restores the original drawn runs"
    );
}

struct LiteralSelectionRoot {
    state: Entity<TextViewState>,
}
impl Render for LiteralSelectionRoot {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui::div()
            .w(px(220.))
            .child(crate::TextSelectionLayer)
            .child(TextView::new(&self.state).selectable(true))
    }
}

#[gpui::test]
fn html_partial_multiblock_drag_keeps_literal_plain_copy_after_find(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let source = "<p>prefix 한글🙂 literal first</p><pre>middle  spaces\n\t tab &amp; &lt;literal&gt;</pre><p>literal ending suffix</p>";
    let (root, cx) = cx.add_window_view(|_, cx| LiteralSelectionRoot {
        state: cx.new(|cx| {
            TextViewState::html(source, cx)
                .rendered_range_support(crate::text::RenderedRangeSupport::HtmlAndMarkdown)
        }),
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let state = root.read_with(cx, |root, _| root.state.clone());
    let bounds = state.read_with(cx, |state, _| state.bounds());
    let start = bounds.origin + point(px(35.), px(4.));
    let end = bounds.bottom_right() - point(px(35.), px(4.));
    cx.simulate_mouse_down(start, gpui::MouseButton::Left, gpui::Modifiers::default());
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.simulate_mouse_move(end, gpui::MouseButton::Left, gpui::Modifiers::default());
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.simulate_mouse_up(end, gpui::MouseButton::Left, gpui::Modifiers::default());
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let selected = state.read_with(cx, |state, _| {
        let selected = state.selected_text();
        assert!(
            selected.contains("middle  spaces\n\t tab & <literal>"),
            "{selected:?}"
        );
        assert!(selected.len() < state.rendered_text().len());
        assert!(state.selected_source_range().is_none());
        selected
    });
    state.update(cx, |state, cx| {
        let text = state.rendered_text();
        let start = text.as_str().find("middle").unwrap();
        state
            .set_range_highlights([highlight(start..start + 6)], cx)
            .unwrap();
        state.reveal_range(start..start + 6, cx).unwrap();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(
        state.read_with(cx, |state, _| state.selected_text()),
        selected
    );
    state.update(cx, |state, cx| state.clear_range_highlights(cx));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(
        state.read_with(cx, |state, _| state.selected_text()),
        selected
    );
}

struct InkRoot {
    state: Entity<TextViewState>,
    ink: Hsla,
}
impl Render for InkRoot {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui::div()
            .w(px(320.))
            .text_color(self.ink)
            .child(TextView::new(&self.state).selectable(true))
    }
}

#[gpui::test]
fn html_drawn_foreground_overlap_updates_and_clear_preserve_fonts_in_both_schemes(
    cx: &mut TestAppContext,
) {
    cx.update(crate::init);
    for (appearance, body, fill) in [
        (
            crate::ThemeAppearance::Light,
            gpui::rgb(0x181818),
            gpui::rgb(0xffde8b),
        ),
        (
            crate::ThemeAppearance::Dark,
            gpui::rgb(0xeeeeee),
            gpui::rgb(0x8c6c1f),
        ),
    ] {
        cx.update(|cx| crate::Theme::global_mut(cx).appearance = appearance);
        let text = "html ink overlap";
        let (root, visual) = cx.add_window_view(|_, cx| InkRoot {
            state: cx.new(|cx| literal_state(text, cx)),
            ink: body.into(),
        });
        visual.run_until_parked();
        visual.update(|window, cx| window.draw(cx).clear(cx));
        let before = painted_runs(text);
        let state = root.read_with(visual, |root, _| root.state.clone());
        for ink in [gpui::rgb(0x21180a), gpui::rgb(0x553300)] {
            state.update(visual, |state, cx| {
                state
                    .set_range_highlights(
                        [
                            RangeHighlight::new(3..8, fill)
                                .with_foreground(Some(gpui::rgb(0xff0000).into())),
                            RangeHighlight::new(0..text.len(), fill)
                                .with_foreground(Some(ink.into())),
                            RangeHighlight::new(2..9, fill),
                        ],
                        cx,
                    )
                    .unwrap();
            });
            visual.update(|window, cx| window.draw(cx).clear(cx));
            let after = painted_runs(text);
            assert!(after.iter().all(|run| run.color == ink.into()));
            assert!(after.iter().all(|run| run.font == before[0].font));
            assert_eq!(after.iter().map(|run| run.len).sum::<usize>(), text.len());
        }
        state.update(visual, |state, cx| state.clear_range_highlights(cx));
        visual.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(painted_runs(text), before);
    }
}
