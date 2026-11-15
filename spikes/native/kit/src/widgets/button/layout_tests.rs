//! Source multiline height versus explicitly preserved published filled padding.
use super::*;
use gpui::{Bounds, Context, Pixels, Render, TestAppContext, Window};
use std::{cell::RefCell, rc::Rc};

#[gpui::test]
fn typed_two_line_labels_add_no_vertical_padding_in_either_theme(cx: &mut TestAppContext) {
    type Seen = Rc<RefCell<Vec<Bounds<Pixels>>>>;
    struct Probe {
        scheme: crate::Scheme,
        seen: Seen,
    }
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
            let tokens = Tokens::of(self.scheme);
            let config = ButtonConfig::new(
                ButtonKind::Ghost,
                ButtonState::Enabled,
                ControlSizing::Pointer,
            );
            let seen = Rc::clone(&self.seen);
            div()
                .flex()
                .flex_col()
                .items_start()
                .child(
                    button_control("md", ButtonContent::Label("A\nB".into()), config, &tokens)
                        .w(px(120.0)),
                )
                .child(
                    button_control(
                        "sm",
                        ButtonContent::Label("A\nB".into()),
                        config.with_size(ButtonSize::Small),
                        &tokens,
                    )
                    .w(px(120.0)),
                )
                .child(
                    button_with_kind(
                        "legacy",
                        "A\nB",
                        ButtonKind::Ghost,
                        ButtonState::Enabled,
                        &tokens,
                    )
                    .w(px(120.0)),
                )
                .on_children_prepainted(move |bounds, _, _| *seen.borrow_mut() = bounds)
        }
    }
    for scheme in [crate::Scheme::Light, crate::Scheme::Dark] {
        let seen = Seen::default();
        let (_, cx) = cx.add_window_view({
            let seen = Rc::clone(&seen);
            move |_, _| Probe { scheme, seen }
        });
        cx.run_until_parked();
        let bounds = seen.borrow();
        let [medium, small, legacy] = bounds.as_slice() else {
            panic!("three actual controls: {bounds:?}");
        };
        assert_eq!(medium.size.height, px(42.0));
        assert_eq!(small.size.height, px(34.0));
        assert_eq!(legacy.size.height, px(50.0));
    }
}
