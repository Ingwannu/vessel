//! Paseo's sheet geometry: 560 on a desktop, the full width on a phone, and
//! rows whose fill reaches 8 past the content inset on both sides.
use super::*;

/// The dialog body's inset (Paseo's sheet padding 24).
const INSET: f32 = 24.0;

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
    assert_eq!(f32::from(header.size.width), 560.0);
    let row = cx.debug_bounds("history-row-codex-native").expect("a row");
    let card_left = f32::from(header.left());
    assert_eq!(f32::from(row.left()) - card_left, INSET - 8.0);
    assert_eq!(
        f32::from(header.right()) - f32::from(row.right()),
        INSET - 8.0
    );
}

#[gpui::test]
fn a_phone_shows_the_sheet_across_the_window(cx: &mut TestAppContext) {
    let (_shell, cx) = listed(cx, 390.0);
    let header = cx
        .debug_bounds("dialog-header")
        .expect("the sheet's header");
    assert_eq!(f32::from(header.size.width), 390.0);
    assert!(cx.debug_bounds("history-row-codex-native").is_some());
}
