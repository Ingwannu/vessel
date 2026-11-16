//! The selectable code row keeps `code_row`'s rails and wraps a long value
//! inside the row instead of widening or clipping it.
use super::*;
use crate::theme::Scheme;
use gpui::{Context, IntoElement, Pixels, Render, TestAppContext, Window};

const SHORT: &str = "00000000-0000-4000-8000-000000000099";
const LONG: &str = "relay-with-a-very-long-host-name-for-the-narrow-sheet.example.invalid:443";

struct Probe {
    width: f32,
    value: &'static str,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        div().child(
            div()
                .debug_selector(|| "probe-row".to_owned())
                .w(px(self.width))
                .child(selectable_code_row(
                    "probe-value",
                    "Machine",
                    self.value,
                    true,
                    &tokens,
                    window,
                    cx,
                )),
        )
    }
}

fn open(
    cx: &mut TestAppContext,
    width: f32,
    value: &'static str,
) -> (gpui::Bounds<Pixels>, gpui::Bounds<Pixels>) {
    cx.update(|cx| crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    let (_, cx) = cx.add_window_view(move |_, _| Probe { width, value });
    cx.run_until_parked();
    (
        cx.debug_bounds("probe-row").unwrap(),
        cx.debug_bounds("settings-row-value").unwrap(),
    )
}

/// The distance from `from` to `to` (`mul_add`: no arithmetic operator).
fn apart(from: Pixels, to: Pixels) -> u32 {
    f32::from(from).mul_add(-1.0, f32::from(to)).to_bits()
}

#[gpui::test]
fn the_value_sits_on_code_rows_rails(cx: &mut TestAppContext) {
    let (row, value) = open(cx, 360.0, SHORT);
    // The row's 16 padding, then the title's 21-point line and 4 under it.
    assert_eq!(apart(row.left(), value.left()), 16.0_f32.to_bits());
    assert_eq!(apart(row.top(), value.top()), 41.0_f32.to_bits());
    assert_eq!(apart(value.right(), row.right()), 16.0_f32.to_bits());
}

#[gpui::test]
fn a_long_value_wraps_inside_a_narrow_row(cx: &mut TestAppContext) {
    let (row, value) = open(cx, 200.0, LONG);
    assert_eq!(row.size.width, px(200.0));
    assert_eq!(apart(value.right(), row.right()), 16.0_f32.to_bits());
    // More than one 20-point code line: wrapped, not cut.
    assert!(value.size.height >= px(40.0), "{value:?}");
}
