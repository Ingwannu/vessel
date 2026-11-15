//! The failed-home regression lives at its actual area composition, not a
//! lookalike kit row. It opens no link, collector, real node or app shell.
use super::*;
use gpui::{Bounds, Context, IntoElement, Pixels, Render, TestAppContext, Window};
use std::{cell::RefCell, rc::Rc};
use vessel_app_kit::theme::Scheme;

struct Probe {
    seen: Rc<RefCell<[Vec<Bounds<Pixels>>; 2]>>,
}
impl Render for Probe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let mut rows = div()
            .w(px(640.0))
            .flex()
            .flex_col()
            .debug_selector(|| "sync-home-row-frame".to_owned());
        for (index, hint) in [
            "A short hint.",
            "A longer hint takes the remaining width. It contains five sentences. The badge stays at the trailing edge. Each sentence adds text without adding a second flex column. Its vertical centre follows the entire row.",
        ].into_iter().enumerate() {
            let seen = Rc::clone(&self.seen);
            rows = rows.child(home_identity(
                if index == 0 { "short-home" } else { "long-home" }.into(), hint.into(), "Not checked", Variant::Muted, false, &tokens,
            ).on_children_prepainted(move |bounds, _window, _cx| {
                if let Some(place) = seen.borrow_mut().get_mut(index) { *place = bounds; }
            }));
        }
        rows
    }
}
#[gpui::test]
fn short_and_five_sentence_hints_keep_the_badge_at_the_same_trailing_edge(cx: &mut TestAppContext) {
    let seen = Rc::new(RefCell::new([Vec::new(), Vec::new()]));
    let (_, cx) = cx.add_window_view({
        let seen = Rc::clone(&seen);
        move |_window, _cx| Probe { seen }
    });
    cx.run_until_parked();
    // Measure the badge itself, not an outer flex wrapper: that wrapper's
    // right edge stayed correct in the original bug while its badge moved.
    let short_badge = cx
        .debug_bounds("sync-home-state-short-home")
        .expect("actual short badge");
    let long_badge = cx
        .debug_bounds("sync-home-state-long-home")
        .expect("actual long badge");
    let frame = cx.debug_bounds("sync-home-row-frame").expect("row frame");
    let bounds = seen.borrow();
    let short = bounds.first().expect("short row");
    let long = bounds.get(1).expect("five-sentence row");
    assert_eq!(short.len(), 2, "words and their one trailing control");
    assert_eq!(long.len(), 2, "words and their one trailing control");
    let short_text = short.first().unwrap();
    let long_text = long.first().unwrap();
    assert_eq!(short_text.left(), long_text.left());
    assert_eq!(short_badge.right(), long_badge.right());
    // The position is relative to the actual frame, not a window-origin
    // assumption. Fractional font advances can round by less than a pixel.
    // float_arithmetic also denies negating a spacing constant; use the
    // workspace convention to subtract that padding without an escape hatch.
    let edge = spacing::S4.mul_add(-1.0, f32::from(frame.right()));
    assert!(
        f32::from(short_badge.right()).mul_add(-1.0, edge).abs() <= 0.01,
        "badge {short_badge:?} must end at the padded edge of {frame:?}"
    );
    assert!(long_text.size.height > short_text.size.height);
    for (text, badge) in [(short_text, &short_badge), (long_text, &long_badge)] {
        assert_eq!(
            text.center().y,
            badge.center().y,
            "one vertically centred row"
        );
    }
}
