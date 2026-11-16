//! Original ADR0100 causal assertions on the shipped GPUI/Kit dependency graph.
//! Observation uses the identical approved change12 read-only retained-run copy.
use gpui::{
    AppContext as _, Context, Entity, ParentElement as _, Render, Styled as _, TestAppContext,
};
use gpui::{Hsla, IntoElement, TextRun, Window, px};
use gpui_base::text::{
    RangeHighlight, RangeHighlightError, SelectionFormat, TextView, TextViewState,
};
use std::ops::Range;

// Implementation later adds only the explicit range opt-in here. Baseline must
// reach the real Unsupported result, rather than fail on a nonexistent API.
fn literal_state(text: &str, cx: &mut Context<TextViewState>) -> TextViewState {
    TextViewState::html(&literal_html(text), cx)
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
    cx.update(gpui_base::init);
    let state = cx.update(|cx| cx.new(|cx| literal_state(text, cx)));
    cx.run_until_parked();
    state
}

fn highlight(range: Range<usize>) -> RangeHighlight {
    RangeHighlight::new(range, gpui::hsla(0.12, 0.8, 0.6, 0.8))
}

#[gpui::test]
#[expect(
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    reason = "literal fixture offsets are explicit valid UTF-8 boundaries"
)]
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
    cx.update(gpui_base::init);
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

#[expect(
    clippy::expect_used,
    reason = "the fixture draws this unique text before reading actual retained runs"
)]
fn painted_runs(text: &str) -> Vec<TextRun> {
    gpui_base::text::retained_text_runs(text)
        .expect("the same drawn TextView must have a retained layout")
}

// Implementation later adds only the optional foreground builder here. The
// baseline still draws a valid background range so a real ink assertion fails.
fn current_highlight(range: Range<usize>, background: Hsla, _foreground: Hsla) -> RangeHighlight {
    RangeHighlight::new(range, background)
}

#[gpui::test]
#[expect(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "the drawn fixture must contain runs covering its complete text"
)]
fn html_rendered_range_contract_current_ink_reaches_actual_retained_runs(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
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
