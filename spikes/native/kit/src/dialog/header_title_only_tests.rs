//! Paseo's combobox sheet header (TRACKER 798): the title alone, 24 across,
//! with no close button; Escape still closes.
use super::*;
use crate::{Scheme, SchemeChoice};
use gpui::{Render, TestAppContext};

struct Probe {
    state: DialogState,
    closed: bool,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let content = DialogContent {
            title: "Workspace tabs".into(),
            body: div().h(px(120.0)).into_any_element(),
            footer: Vec::new(),
            size: DialogSize::Normal,
        }
        .with_title_only();
        let modal = dialog(
            &self.state,
            content,
            &tokens,
            window,
            true,
            |probe: &mut Self, _, _| probe.closed = true,
            cx,
        );
        div().size_full().child(modal)
    }
}

#[gpui::test]
fn the_title_only_header_has_no_close_button_and_escape_closes(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    let (view, cx) = cx.add_window_view(|_window, cx| Probe {
        state: DialogState::new(cx),
        closed: false,
    });
    view.update_in(cx, |probe, window, cx| probe.state.focus(window, cx));
    cx.run_until_parked();
    let header = cx.debug_bounds("dialog-header").unwrap();
    let title = cx.debug_bounds("dialog-title").unwrap();
    assert!(cx.debug_bounds("dialog-close").is_none(), "no close button");
    let inset = f32::from(header.left()).mul_add(-1.0, f32::from(title.left()));
    assert_eq!(inset.to_bits(), 24.0_f32.to_bits(), "24 across");
    cx.simulate_keystrokes("escape");
    assert!(
        view.read_with(cx, |probe, _| probe.closed),
        "Escape still closes"
    );
}
