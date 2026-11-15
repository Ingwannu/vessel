//! Real arbitrary input, measured anchoring and retained-render dismissal paths.

use super::*;
use crate::{Scheme, SchemeChoice, TextField, Tokens};
use gpui::prelude::*;
use gpui::{
    Entity, FocusHandle, Modifiers, Render, TestAppContext, VisualTestContext, div, point, px, size,
};
use std::cell::RefCell;

struct Probe {
    state: PopoverState,
    outside: TextField,
    inside: TextField,
    newer: FocusHandle,
    narrow: bool,
    pending: bool,
    context: u8,
    admissions: usize,
    body: Rc<RefCell<Vec<Bounds<Pixels>>>>,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let context = self.context;
        let measured = Rc::clone(&self.body);
        let content = div()
            .w_full()
            .on_children_prepainted(move |bounds, _, _| *measured.borrow_mut() = bounds)
            .child(div().w_full().h(px(1.0)).flex_shrink_0())
            .child(self.inside.element(&tokens, false, cx));
        div()
            .size_full()
            .child(self.outside.element(&tokens, false, cx))
            .child(div().track_focus(&self.newer))
            .children(popover(
                &self.state,
                PopoverContent::new(
                    "adaptive-options",
                    "Options",
                    content.into_any_element(),
                    Width::new(340).unwrap(),
                ),
                &PopoverHandlers {
                    state: |probe| &mut probe.state,
                    admit: Rc::new(move |probe, _lease, _, _| {
                        probe.admissions = probe.admissions.saturating_add(1);
                        if probe.pending || probe.context != context {
                            Dismissal::KeepOpen
                        } else {
                            Dismissal::Close
                        }
                    }),
                },
                &tokens,
                window,
                self.narrow,
                cx,
            ))
    }
}

fn anchor() -> Bounds<Pixels> {
    Bounds::new(point(px(600.0), px(100.0)), size(px(40.0), px(28.0)))
}

fn mount(cx: &mut TestAppContext, narrow: bool) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap();
        cx.set_reduce_motion(true);
    });
    let (probe, cx) = cx.add_window_view(move |window, cx| {
        window.activate_window();
        let outside = TextField::new("", |_, _, _, _| {}, window, cx);
        let inside = TextField::new("", |_, _, _, _| {}, window, cx);
        outside.focus(window, cx);
        let mut state = PopoverState::new(cx);
        let _lease = state.open(anchor(), Alignment::End, window, cx);
        Probe {
            state,
            outside,
            inside,
            newer: cx.focus_handle(),
            narrow,
            pending: false,
            context: 1,
            admissions: 0,
            body: Rc::default(),
        }
    });
    cx.simulate_resize(size(px(if narrow { 390.0 } else { 1280.0 }), px(900.0)));
    cx.run_until_parked();
    (probe, cx)
}

#[derive(Debug, Clone, Copy)]
enum Path {
    Escape,
    Outside,
    Glyph,
}

fn activate(path: Path, cx: &mut VisualTestContext) {
    match path {
        Path::Escape => cx.simulate_keystrokes("escape"),
        Path::Outside => {
            cx.simulate_click(point(px(2.0), px(2.0)), Modifiers::none());
        }
        Path::Glyph => {
            let at = cx.debug_bounds("dialog-close").unwrap().center();
            cx.simulate_click(at, Modifiers::none());
        }
    }
}

#[gpui::test]
fn arbitrary_field_keeps_input_and_escape_returns_to_the_actual_opener(cx: &mut TestAppContext) {
    for narrow in [false, true] {
        let (probe, cx) = mount(cx, narrow);
        probe.update_in(cx, |probe, window, cx| probe.inside.focus(window, cx));
        cx.simulate_input("retained input");
        cx.run_until_parked();
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        probe.update_in(cx, |probe, window, cx| {
            assert!(!probe.state.is_open());
            assert_eq!(probe.inside.text(cx), "retained input");
            assert!(probe.outside.focus_handle(cx).is_focused(window));
            assert_eq!(probe.admissions, 1);
        });
    }
}

#[gpui::test]
fn desktop_is_end_anchored_340_and_compact_reuses_the_existing_dialog(cx: &mut TestAppContext) {
    for (narrow, expected_width) in [(false, 306.0), (true, 342.0)] {
        let (probe, cx) = mount(cx, narrow);
        probe.read_with(cx, |probe, _| {
            let measured = probe.body.borrow();
            let marker = measured.first().unwrap();
            assert_eq!(marker.size.width, px(expected_width));
            if !narrow {
                assert_eq!(marker.origin.x, px(317.0));
                assert_eq!(marker.origin.y, px(133.0));
            }
        });
    }
}

#[gpui::test]
fn pending_and_replaced_openings_refuse_every_old_rendered_dismissal_before_focus_ack(
    cx: &mut TestAppContext,
) {
    for narrow in [false, true] {
        let paths = if narrow {
            vec![Path::Escape, Path::Outside, Path::Glyph]
        } else {
            vec![Path::Escape, Path::Outside]
        };
        for path in paths {
            let (probe, cx) = mount(cx, narrow);
            probe.update_in(cx, |probe, window, cx| {
                probe.pending = true;
                probe.inside.focus(window, cx);
            });
            activate(path, cx);
            probe.update_in(cx, |probe, window, cx| {
                assert!(probe.state.is_open());
                assert!(probe.inside.focus_handle(cx).is_focused(window));
                probe.pending = false;
                probe.admissions = 0;
                probe.context = 2;
                let _new = probe.state.open(anchor(), Alignment::End, window, cx);
                // Do not render B: the retained input handlers still belong to A.
            });
            activate(path, cx);
            probe.update_in(cx, |probe, window, _cx| {
                assert!(probe.state.is_open(), "{narrow:?}/{path:?}");
                assert!(probe.state.dialog.focus_handle().is_focused(window));
                assert_eq!(
                    probe.admissions, 0,
                    "stale input never reaches domain admission"
                );
            });
            cx.run_until_parked();
            activate(path, cx);
            probe.update_in(cx, |probe, window, cx| {
                assert!(!probe.state.is_open());
                assert!(probe.outside.focus_handle(cx).is_focused(window));
                assert_eq!(probe.admissions, 1);
            });
        }
    }
}

#[gpui::test]
fn explicit_close_preserves_newer_focus_and_stale_receipt_cannot_close_reopen(
    cx: &mut TestAppContext,
) {
    let (probe, cx) = mount(cx, false);
    probe.update_in(cx, |probe, window, cx| {
        let old = probe.state.open(anchor(), Alignment::Start, window, cx);
        let current = probe.state.open(anchor(), Alignment::End, window, cx);
        assert!(!probe.state.close(&old, window, cx));
        assert!(probe.state.is_current(&current));
        probe.newer.focus(window, cx);
        assert!(probe.state.close(&current, window, cx));
        assert!(probe.newer.is_focused(window));
        assert!(!probe.state.close(&current, window, cx));
    });
}
