//! Real input-state and layout checks, shared with the composer's handler.

use std::cell::RefCell;

use gpui::{Bounds, EntityInputHandler as _, Pixels, Render, TestAppContext};

use super::*;
use crate::theme::Scheme;

struct Probe {
    field: TextField,
    narrow: bool,
    seen: Rc<RefCell<Vec<Bounds<Pixels>>>>,
    submitted: usize,
    dialog: Option<crate::DialogState>,
    rendered_text: String,
}

impl Probe {
    fn new(
        style: MultilineStyle,
        narrow: bool,
        seen: Rc<RefCell<Vec<Bounds<Pixels>>>>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let on_submit: fn(&mut Self, String, &mut Window, &mut Context<'_, Self>) =
            |owner, _, _, _| {
                owner.submitted = owner.submitted.saturating_add(1);
            };
        let field = match style {
            MultilineStyle::Composer => TextField::composer("Message", on_submit, window, cx),
            MultilineStyle::Form => TextField::form_multiline("Message", on_submit, window, cx),
        };
        field.focus(window, cx);
        Self {
            field,
            narrow,
            seen,
            submitted: 0,
            dialog: None,
            rendered_text: String::new(),
        }
    }

    fn input(&self) -> Entity<TextareaState> {
        match &self.field.kind {
            Kind::Lines(state, _) => state.clone(),
            Kind::Line(_) => panic!("the probe is a multiline field"),
        }
    }
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.rendered_text = self.field.text(cx);
        let seen = Rc::clone(&self.seen);
        let tokens = Tokens::of(Scheme::Dark);
        let body = div()
            .w_full()
            .max_w(px(360.0))
            .on_children_prepainted(move |bounds, _, _| *seen.borrow_mut() = bounds)
            .child(self.field.element(&tokens, self.narrow, cx));
        if let Some(state) = &self.dialog {
            crate::dialog(
                state,
                crate::DialogContent {
                    title: "Message".into(),
                    body: body.into_any_element(),
                    footer: Vec::new(),
                    size: crate::DialogSize::Normal,
                },
                &tokens,
                window,
                self.narrow,
                |_, _, _| {},
                cx,
            )
        } else {
            body.into_any_element()
        }
    }
}

#[gpui::test]
fn the_form_grows_then_scrolls_without_losing_its_text(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    for narrow in [false, true] {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let (view, cx) = cx.add_window_view({
            let seen = Rc::clone(&seen);
            move |window, cx| Probe::new(MultilineStyle::Form, narrow, seen, window, cx)
        });
        let mut heights = Vec::new();
        for lines in [1, 6, 20, 30] {
            let text = vec!["line"; lines].join("\n");
            cx.update(|window, cx| {
                view.update(cx, |probe, cx| {
                    probe.field.set_text(&text, window, cx);
                    let state = probe.input();
                    state.update(cx, |state, cx| {
                        let end = state.value().len();
                        state.set_selected_range(end..end, cx);
                    });
                    cx.notify();
                });
            });
            cx.run_until_parked();
            heights.push(
                seen.borrow()
                    .first()
                    .expect("the field was laid out")
                    .size
                    .height,
            );
            cx.update(|_, cx| {
                view.read_with(cx, |probe, cx| {
                    assert_eq!(
                        probe.field.text(cx),
                        text,
                        "a viewport cap must not truncate text"
                    );
                    if lines == 30 {
                        assert!(
                            probe.input().read(cx).scroll_offset().y < px(0.0),
                            "the last line must be scrolled into view"
                        );
                    }
                });
            });
        }
        assert!(
            heights[1] > heights[0],
            "six lines must grow beyond the minimum"
        );
        assert!(heights[2] >= heights[1]);
        assert_eq!(
            heights[2], heights[3],
            "more than twelve lines must scroll instead of growing"
        );
        assert!(heights[3] <= px(FORM_MAX));
    }
}

#[gpui::test]
fn composer_and_form_pass_hangul_marks_and_commits_to_the_same_handler(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    for style in [MultilineStyle::Composer, MultilineStyle::Form] {
        let (view, cx) = cx.add_window_view(move |window, cx| {
            Probe::new(style, false, Rc::new(RefCell::new(Vec::new())), window, cx)
        });
        cx.update(|window, cx| {
            view.update(cx, |probe, cx| {
                let input = probe.input();
                input.update(cx, |state, cx| {
                    for mark in ["ㅎ", "하", "한"] {
                        state.replace_and_mark_text_in_range(None, mark, Some(1..1), window, cx);
                        assert_eq!(state.value(), mark);
                        assert_eq!(state.marked_text_range(window, cx), Some(0..1));
                    }
                    state.replace_text_in_range(None, "한", window, cx);
                    assert_eq!(state.marked_text_range(window, cx), None);
                    for mark in ["ㄱ", "그", "글"] {
                        state.replace_and_mark_text_in_range(None, mark, Some(1..1), window, cx);
                        assert_eq!(state.marked_text_range(window, cx), Some(1..2));
                    }
                    state.replace_text_in_range(None, "글", window, cx);
                    state.unmark_text(window, cx);
                    assert_eq!(state.marked_text_range(window, cx), None);
                });
                assert_eq!(probe.field.text(cx), "한글");
                assert_eq!(probe.submitted, 0, "composition must not submit the form");
            });
        });
    }
}

#[gpui::test]
fn enter_starts_a_line_in_a_form_and_mod_enter_submits(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    let (view, cx) = cx.add_window_view(|window, cx| {
        Probe::new(
            MultilineStyle::Form,
            false,
            Rc::new(RefCell::new(Vec::new())),
            window,
            cx,
        )
    });
    cx.run_until_parked();
    cx.simulate_input("first");
    cx.simulate_keystrokes("enter");
    cx.simulate_input("second");
    cx.run_until_parked();
    cx.update(|_, cx| {
        view.read_with(cx, |probe, cx| {
            assert_eq!(probe.field.text(cx), "first\nsecond");
            assert_eq!(probe.submitted, 0);
        });
    });
    let submit = cx.read(|cx| {
        if cx.platform_info().is_mac() {
            "cmd-enter"
        } else {
            "ctrl-enter"
        }
    });
    cx.simulate_keystrokes(submit);
    cx.run_until_parked();
    cx.update(|_, cx| {
        view.read_with(cx, |probe, _| assert_eq!(probe.submitted, 1));
    });
}

#[gpui::test]
fn clicking_a_form_focuses_its_input_inside_and_outside_a_dialog(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    for (width, narrow) in [(1280.0, false), (390.0, true)] {
        for nested in [false, true] {
            let seen = Rc::new(RefCell::new(Vec::new()));
            let (view, cx) = cx.add_window_view({
                let seen = Rc::clone(&seen);
                move |window, cx| {
                    let mut probe = Probe::new(MultilineStyle::Form, narrow, seen, window, cx);
                    if nested {
                        probe.dialog = Some(crate::DialogState::new(cx));
                    }
                    probe
                }
            });
            cx.simulate_resize(gpui::size(px(width), px(800.0)));
            cx.run_until_parked();
            cx.update(Window::blur);
            cx.run_until_parked();
            let bounds = *seen.borrow().first().expect("the field was laid out");
            cx.update(|_, cx| {
                view.read_with(cx, |probe, cx| {
                    let bounds = probe.input().read(cx).input_bounds();
                    assert!(bounds.size.width > px(0.0));
                    assert!(bounds.size.height > px(0.0));
                });
            });
            cx.simulate_click(bounds.center(), gpui::Modifiers::none());
            cx.run_until_parked();
            cx.simulate_input("typed");
            cx.run_until_parked();
            cx.update(|window, cx| {
                view.read_with(cx, |probe, cx| {
                    assert!(
                        probe.field.focus_handle(cx).is_focused(window),
                        "the click must focus the input"
                    );
                    assert_eq!(
                        probe.rendered_text, "typed",
                        "the form owner must see edits"
                    );
                    assert_eq!(
                        probe.field.text(cx),
                        "typed",
                        "a focused field must accept typing"
                    );
                });
            });
        }
    }
}

/// The caret blinks while things may move, and is steady while motion is
/// held (the patched gpui-base, `spikes/release/patches/README.md`): every
/// blink notifies the field, and a notified field repaints the window
/// (about 31 % of a core at 1280 × 800 on a software renderer).
#[gpui::test]
fn the_caret_is_steady_while_motion_is_held(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    for held in [false, true] {
        cx.update(|cx| cx.set_reduce_motion(held));
        let (view, cx) = cx.add_window_view(|window, cx| {
            Probe::new(
                MultilineStyle::Composer,
                false,
                Rc::new(RefCell::new(Vec::new())),
                window,
                cx,
            )
        });
        cx.run_until_parked();
        // The window made active and the field focused again, as a click
        // does: a test window starts inactive, and a focus event reaches
        // the field's listener only in an active window.
        cx.update(|window, _cx| window.activate_window());
        cx.run_until_parked();
        cx.update(Window::blur);
        cx.run_until_parked();
        let handle = cx.update(|_window, cx| view.read(cx).field.focus_handle(cx));
        cx.update(|window, cx| handle.focus(window, cx));
        cx.run_until_parked();
        let input = view.read_with(cx, |probe, _cx| probe.input());
        let notified = Rc::new(Cell::new(0_u32));
        let count = Rc::clone(&notified);
        let _watch = cx.update(|_window, cx| {
            cx.observe(&input, move |_input, _cx| {
                count.set(count.get().saturating_add(1));
            })
        });
        for _ in 0..6 {
            cx.executor()
                .advance_clock(std::time::Duration::from_millis(500));
            cx.run_until_parked();
        }
        if held {
            assert_eq!(notified.get(), 0, "held: the caret does not blink");
        } else {
            assert!(
                notified.get() >= 3,
                "moving: it blinks ({})",
                notified.get()
            );
        }
    }
}
