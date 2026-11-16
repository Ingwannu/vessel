//! Text in a dialog can be selected: a press on the card reaches the
//! window's selection layer, still does not close the dialog, and does not
//! take focus from a field inside it.
use super::*;
use crate::{Scheme, SchemeChoice};
use gpui::{Modifiers, Render, TestAppContext};

const VALUE: &str = "00000000-0000-4000-8000-000000000099";

struct Probe {
    state: DialogState,
    field: crate::TextField,
    closes: u32,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let body = div()
            .debug_selector(|| "dialog-probe-value".to_owned())
            .child(crate::plain_text(
                "dialog-probe-value",
                VALUE,
                0,
                window,
                cx,
            ))
            .into_any_element();
        let content = DialogContent {
            title: "Machine".into(),
            body,
            footer: Vec::new(),
            size: DialogSize::Normal,
        };
        let modal = dialog(
            &self.state,
            content,
            &tokens,
            window,
            false,
            |probe: &mut Self, _, _| probe.closes = probe.closes.saturating_add(1),
            cx,
        );
        div()
            .size_full()
            .child(gpui_base::TextSelectionLayer)
            .child(modal)
    }
}

#[gpui::test]
fn a_drag_over_dialog_text_selects_it_without_closing(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    let (view, cx) = cx.add_window_view(|window, cx| {
        window.activate_window();
        Probe {
            state: DialogState::new(cx),
            field: crate::TextField::new("", |_, _, _, _| {}, window, cx),
            closes: 0,
        }
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        let _paint = window.draw(cx);
    });
    let value = cx.debug_bounds("dialog-probe-value").unwrap();
    let start = point(right_of(value.left(), 2.0), value.center().y);
    let end = point(right_of(value.left(), 120.0), value.center().y);
    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    cx.update(|window, cx| {
        let _paint = window.draw(cx);
        assert!(gpui_base::TextSelection::has_selection(window, cx));
        assert!(!gpui_base::TextSelection::selected_text(window, cx).is_empty());
    });
    view.read_with(cx, |probe, _| assert_eq!(probe.closes, 0));
}

/// `by` to the right of `from` (`mul_add`: no arithmetic operator).
fn right_of(from: gpui::Pixels, by: f32) -> gpui::Pixels {
    px(f32::from(from).mul_add(1.0, by))
}

#[gpui::test]
fn a_press_on_the_card_keeps_a_focused_field_focused(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    let (view, cx) = cx.add_window_view(|window, cx| {
        window.activate_window();
        Probe {
            state: DialogState::new(cx),
            field: crate::TextField::new("", |_, _, _, _| {}, window, cx),
            closes: 0,
        }
    });
    cx.run_until_parked();
    view.update_in(cx, |probe, window, cx| probe.field.focus(window, cx));
    cx.run_until_parked();
    // The title and the close glyph's row: card, not a control that takes focus.
    let title = cx.debug_bounds("dialog-title").unwrap();
    cx.simulate_click(title.center(), Modifiers::none());
    cx.run_until_parked();
    view.update_in(cx, |probe, window, cx| {
        assert!(probe.field.focus_handle(cx).is_focused(window));
        assert_eq!(probe.closes, 0);
    });
}
