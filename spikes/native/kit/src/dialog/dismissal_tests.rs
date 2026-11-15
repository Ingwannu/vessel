//! All three existing dismissal paths use the closure captured by this dialog.
//! The kit does not interpret its token; feature policy remains with the caller.
use super::*;
use crate::theme::Scheme;
use gpui::{Modifiers, Render, TestAppContext};
use std::{cell::RefCell, rc::Rc};

struct Probe {
    state: DialogState,
    token: Rc<str>,
    closed: Rc<RefCell<Vec<Rc<str>>>>,
}
impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let token = Rc::clone(&self.token);
        dialog(
            &self.state,
            DialogContent {
                title: "A decision".into(),
                body: div()
                    .child("Confirm the original decision.")
                    .into_any_element(),
                footer: Vec::new(),
                size: DialogSize::Normal,
            },
            &Tokens::of(Scheme::Dark),
            window,
            false,
            move |probe, _, _| probe.closed.borrow_mut().push(Rc::clone(&token)),
            cx,
        )
    }
}

#[gpui::test]
fn escape_scrim_and_close_glyph_retain_the_callers_captured_decision(cx: &mut TestAppContext) {
    #[derive(Debug)]
    enum Dismissal {
        Escape,
        Scrim,
        Glyph,
    }
    for path in [Dismissal::Escape, Dismissal::Scrim, Dismissal::Glyph] {
        let closed = Rc::new(RefCell::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let closed = Rc::clone(&closed);
            move |window, cx| {
                let state = DialogState::new(cx);
                state.focus(window, cx);
                Probe {
                    state,
                    token: Rc::from("the original decision"),
                    closed,
                }
            }
        });
        cx.run_until_parked();
        match path {
            Dismissal::Escape => cx.simulate_keystrokes("escape"),
            Dismissal::Scrim => cx.simulate_click(point(px(0.0), px(0.0)), Modifiers::default()),
            Dismissal::Glyph => {
                let bounds = cx
                    .debug_bounds("dialog-close")
                    .expect("the actual close glyph");
                cx.simulate_click(bounds.center(), Modifiers::default());
            }
        }
        assert_eq!(
            &*closed.borrow(),
            &[Rc::<str>::from("the original decision")],
            "{path:?}"
        );
    }
}
