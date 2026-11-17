//! Paseo's sheet geometry: 560 on a desktop, the full width on a phone, and
//! rows whose fill reaches 8 past the content inset on both sides.
use super::*;
use gpui::Pixels;

/// A row's edge from the card's: the body's 24 inset less the row's 8.
const ROW_FROM_EDGE: f32 = 16.0;
/// The same distance back from the card's right edge.
const ROW_FROM_RIGHT: f32 = -16.0;

/// `at` moved by `by`, without float arithmetic in the assertion.
fn moved(at: Pixels, by: f32) -> Pixels {
    px(f32::from(at).mul_add(1.0, by))
}

fn listed(cx: &mut TestAppContext, width: f32) -> (gpui::Entity<Shell>, &mut VisualTestContext) {
    let (shell, cx, mut commands) = shell(cx, vec![machine("laptop", &[codex()])]);
    cx.simulate_resize(size(px(width), px(900.0)));
    cx.run_until_parked();
    open(&shell, cx);
    let asks = sent(&mut commands);
    answer(&shell, cx, asks, |_| vec![entry("native", 1)]);
    (shell, cx)
}

#[gpui::test]
fn the_desktop_card_is_560_and_rows_reach_8_past_the_inset(cx: &mut TestAppContext) {
    let (_shell, cx) = listed(cx, 1280.0);
    let header = cx
        .debug_bounds("dialog-header")
        .expect("the sheet's header");
    assert_eq!(header.size.width, px(560.0));
    let row = cx.debug_bounds("history-row-codex-native").expect("a row");
    assert_eq!(row.left(), moved(header.left(), ROW_FROM_EDGE));
    assert_eq!(row.right(), moved(header.right(), ROW_FROM_RIGHT));
}

#[gpui::test]
fn a_phone_shows_the_sheet_across_the_window(cx: &mut TestAppContext) {
    let (_shell, cx) = listed(cx, 390.0);
    let header = cx
        .debug_bounds("dialog-header")
        .expect("the sheet's header");
    assert_eq!(header.size.width, px(390.0));
    assert!(cx.debug_bounds("history-row-codex-native").is_some());
}
