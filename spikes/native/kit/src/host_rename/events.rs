//! The shared action must release to its caller once, then isolate its parent.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{Bounds, Context, MouseButton, Pixels, Render, TestAppContext, Window, div, px};

use super::{geometry, host_rename_button};
use crate::theme::spacing;
use crate::{ButtonState, Scheme, Tokens};

type Seen = Rc<RefCell<Vec<Bounds<Pixels>>>>;
const ROW_WIDTH: f32 = 100.0;

#[derive(Default)]
struct Counts {
    rename: Cell<usize>,
    parent_up: Cell<usize>,
    neighbour: Cell<usize>,
}

struct Probe {
    state: ButtonState,
    counts: Rc<Counts>,
    seen: Seen,
}

impl Render for Probe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let called = Rc::clone(&self.counts);
        let rename = host_rename_button("rename", self.state, &tokens).when(
            self.state == ButtonState::Enabled,
            |button| {
                button.on_click(move |_, _, _| {
                    called.rename.set(called.rename.get().saturating_add(1));
                })
            },
        );
        let called = Rc::clone(&self.counts);
        let neighbour = div()
            .id("neighbour")
            .size(geometry().visual)
            .on_click(move |_, _, _| {
                called
                    .neighbour
                    .set(called.neighbour.get().saturating_add(1));
            });
        let called = Rc::clone(&self.counts);
        let seen = Rc::clone(&self.seen);
        div()
            .w(px(ROW_WIDTH))
            .h(px(crate::breakpoint::TOUCH_TARGET))
            .p(px(spacing::S2))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(spacing::S1))
            .on_mouse_up(MouseButton::Left, move |_, _, _| {
                called
                    .parent_up
                    .set(called.parent_up.get().saturating_add(1));
            })
            .on_children_prepainted(move |bounds, _, _| *seen.borrow_mut() = bounds)
            .id("parent")
            .child(rename)
            .child(neighbour)
    }
}

#[gpui::test]
fn rename_consumes_release_after_its_caller_and_leaves_outside_neighbour_reachable(
    cx: &mut TestAppContext,
) {
    for state in [ButtonState::Enabled, ButtonState::Disabled] {
        let counts = Rc::new(Counts::default());
        let seen = Seen::default();
        let (_, cx) = cx.add_window_view({
            let counts = Rc::clone(&counts);
            let seen = Rc::clone(&seen);
            move |_, _| Probe {
                state,
                counts,
                seen,
            }
        });
        cx.run_until_parked();
        let rename = seen.borrow().first().expect("rename laid out").center();
        let neighbour = seen.borrow().get(1).expect("neighbour laid out").center();
        cx.simulate_click(rename, gpui::Modifiers::none());
        cx.run_until_parked();
        match state {
            ButtonState::Enabled => {
                assert_eq!(counts.rename.get(), 1, "the caller must still run once");
                assert_eq!(counts.parent_up.get(), 0, "rename must consume its release");
            }
            ButtonState::Disabled => {
                assert_eq!(counts.rename.get(), 0);
                assert_eq!(
                    counts.parent_up.get(),
                    1,
                    "disabled installs no release handler"
                );
            }
        }
        let renamed = counts.rename.get();
        cx.simulate_click(neighbour, gpui::Modifiers::none());
        cx.run_until_parked();
        assert_eq!(
            counts.neighbour.get(),
            1,
            "an outside click must reach the neighbour"
        );
        assert_eq!(counts.rename.get(), renamed, "outside must not rename");
    }
}
