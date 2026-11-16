//! A dialog's title wraps through GPUI's line wrapper, and the vendored
//! wrapper never starts a line with `?` (gpui-pre change 10). An English
//! confirmation title used to leave its `?` alone on the next line
//! (TRACKER 772). The test text system's monospaced font is 8 points a
//! character at 16, so each width below sits between two character counts.

use gpui::{IndentAdjustment, LineFragment, TestAppContext, font, px};

/// Where `text` wraps at `width` points: the byte offset where each new
/// line starts.
fn wraps(cx: &TestAppContext, text: &str, width: f32) -> Vec<usize> {
    let mut wrapper = cx.text_system().line_wrapper(font(".ZedMono"), px(16.0));
    wrapper
        .wrap_line(
            &[LineFragment::text(text)],
            px(width),
            IndentAdjustment::default(),
        )
        .map(|boundary| boundary.ix)
        .collect()
}

#[gpui::test]
fn a_question_mark_stays_with_its_word(cx: &mut TestAppContext) {
    // "Remove this device" fits in 148 points (18 characters); its `?`
    // does not, so the whole last word moves down with it.
    assert_eq!(wraps(cx, "Remove this device?", 148.0), vec![12]);
}

#[gpui::test]
fn a_url_still_wraps_after_its_question_mark(cx: &mut TestAppContext) {
    // Ten characters fit in 84 points; the query wraps after its `?`.
    assert_eq!(wraps(cx, "aaaaaa?bbbbbb", 84.0), vec![7]);
}
