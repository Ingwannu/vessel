//! The toast card in Paseo's boundary (TRACKER 803, `toast-host.tsx`
//! `widthBoundary`: 92% of the window less 16 each side, at most 480): a
//! long message wraps inside it onto more lines, a short one stays on one.
use super::*;
use crate::{Scheme, SchemeChoice};
use gpui::{Bounds, Context, Pixels, Render, TestAppContext, size};

struct Probe {
    text: SharedString,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let card = toast(
            "toast",
            self.text.clone(),
            ToastLook::Error,
            &tokens,
            window,
        );
        div().size_full().child(toast_layer(card, 40.0, window))
    }
}

/// The node's words for a reload refused while a turn ignored its cancel.
const LONG: &str = "the session's agent cannot be reloaded: the running turn did not end \
                    after its cancel; it goes on: the running turn did not end after its \
                    cancel; it goes on";

fn card(cx: &mut TestAppContext, text: &'static str, width: f32) -> Bounds<Pixels> {
    cx.update(|cx| {
        crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap();
        cx.set_reduce_motion(true);
    });
    let (_view, cx) = cx.add_window_view(move |_, _| Probe { text: text.into() });
    cx.simulate_resize(size(px(width), px(800.0)));
    cx.run_until_parked();
    cx.debug_bounds("toast-card").unwrap()
}

#[gpui::test]
fn a_long_message_wraps_within_480_on_a_wide_window(cx: &mut TestAppContext) {
    let card = card(cx, LONG, 1280.0);
    let wide = f32::from(card.size.width);
    let tall = f32::from(card.size.height);
    assert!(wide <= 480.5, "at most 480 across, was {wide}");
    assert!(
        wide >= 400.0,
        "it fills the boundary before it wraps, was {wide}"
    );
    assert!(tall >= 60.0, "more than one line, was {tall}");
}

#[gpui::test]
fn a_long_message_keeps_to_92_percent_on_a_phone(cx: &mut TestAppContext) {
    let card = card(cx, LONG, 390.0);
    let wide = f32::from(card.size.width);
    // 92% of 390 less 16 each side.
    assert!(wide <= 330.0, "within the phone's boundary, was {wide}");
}

#[gpui::test]
fn a_short_message_stays_on_one_line_as_wide_as_its_words(cx: &mut TestAppContext) {
    let card = card(cx, "Reloaded agent", 1280.0);
    let wide = f32::from(card.size.width);
    let tall = f32::from(card.size.height);
    assert!(wide < 300.0, "as wide as its words, was {wide}");
    assert!(tall < 45.0, "one line, was {tall}");
}
