//! Actual profile bounds/alignment, reusing the same line engine.
use super::*;
use crate::Scheme;
use gpui::{Bounds, Context, Pixels, Render, TestAppContext, Window};
use std::{cell::RefCell, rc::Rc};
type Seen = Rc<RefCell<Vec<Bounds<Pixels>>>>;
struct Probe {
    field: TextField,
    profile: AppearanceProfile,
    scheme: Scheme,
    seen: Seen,
    submitted: Vec<String>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let seen = Rc::clone(&self.seen);
        div()
            .w(px(390.0))
            .flex()
            .flex_row()
            .on_children_prepainted(move |bounds, _, _| *seen.borrow_mut() = bounds)
            .child(self.field.appearance_element(
                self.profile,
                "Font size".into(),
                &Tokens::of(self.scheme),
                cx,
            ))
    }
}
#[gpui::test]
fn profiles_keep_their_reference_width_minimum_and_engine_alignment(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    for scheme in [Scheme::Light, Scheme::Dark] {
        for (profile, width, alignment) in [
            (AppearanceProfile::Family, 280.0, TextAlign::Left),
            (AppearanceProfile::Number, 64.0, TextAlign::Right),
            (AppearanceProfile::Width, 80.0, TextAlign::Right),
        ] {
            let seen = Seen::default();
            let (probe, cx) = cx.add_window_view({
                let seen = Rc::clone(&seen);
                move |window, cx| Probe {
                    field: TextField::new("", |_: &mut Probe, _, _, _| {}, window, cx),
                    profile,
                    scheme,
                    seen,
                    submitted: Vec::new(),
                }
            });
            probe.update_in(cx, |owner, window, cx| {
                owner.field.set_text("14", window, cx);
            });
            cx.run_until_parked();
            let bounds = seen.borrow();
            assert_eq!(bounds.len(), 1);
            assert_eq!(bounds[0].size.width, px(width));
            assert_eq!(bounds[0].size.height, px(36.0));
            probe.read_with(cx, |owner, cx| {
                let Kind::Line(state) = &owner.field.kind else {
                    panic!("profile is a line");
                };
                assert_eq!(state.read(cx).presentation().text_align(), alignment);
            });
        }
    }
}

// A single-line profile must retain the engine's one-submit path. Multiline
// forms separately stop the propagated Enter action after inserting a line.
#[gpui::test]
fn enter_submits_the_current_line_once_without_inserting_text(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    for profile in [
        AppearanceProfile::Family,
        AppearanceProfile::Number,
        AppearanceProfile::Width,
    ] {
        let (probe, cx) = cx.add_window_view(move |window, cx| Probe {
            field: TextField::new(
                "",
                |owner: &mut Probe, value, _, _| owner.submitted.push(value),
                window,
                cx,
            ),
            profile,
            scheme: Scheme::Dark,
            seen: Seen::default(),
            submitted: Vec::new(),
        });
        cx.update(|window, _| window.activate_window());
        probe.update_in(cx, |owner, window, cx| owner.field.focus(window, cx));
        cx.run_until_parked();
        cx.simulate_input("14");
        cx.run_until_parked();
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        probe.read_with(cx, |owner, cx| {
            assert_eq!(owner.submitted, ["14"]);
            assert_eq!(owner.field.text(cx), "14");
        });
    }
}
