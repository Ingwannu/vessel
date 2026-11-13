//! Real number-input selection, editing and commit event paths.
use super::*;
use gpui::{Context, EntityInputHandler as _, IntoElement, Render, TestAppContext};
struct Probe {
    number: NumberField,
    other: TextField,
    commits: Vec<String>,
    edits: Vec<String>,
}
impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(crate::Scheme::Dark);
        div()
            .flex()
            .flex_col()
            .child(self.number.element(&tokens, cx))
            .child(self.other.element(&tokens, false, cx))
    }
}

#[gpui::test]
fn a_number_field_is_seen_by_the_shared_composition_query(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(crate::Scheme::Dark)).unwrap();
    });
    let (view, cx) = cx.add_window_view(|window, cx| Probe {
        number: NumberField::new(
            NumberWidth::FontSize,
            |_: &mut Probe, _, _, _| {},
            |_: &mut Probe, _, _, _| {},
            window,
            cx,
        ),
        other: TextField::new("", |_: &mut Probe, _, _, _| {}, window, cx),
        commits: Vec::new(),
        edits: Vec::new(),
    });
    cx.run_until_parked();
    view.update_in(cx, |view, window, cx| {
        view.number.field.focus(window, cx);
        let Kind::Line(input) = &view.number.field.kind else {
            panic!("a number field uses the line engine");
        };
        input.update(cx, |input, cx| {
            input.replace_and_mark_text_in_range(None, "1", None, window, cx);
        });
        assert!(TextField::focused_composing(window, cx));
        input.update(cx, |input, cx| input.unmark_text(window, cx));
        assert!(!TextField::focused_composing(window, cx));
    });
}
#[gpui::test]
fn focus_selects_all_and_enter_and_blur_each_commit_the_current_draft(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(crate::Scheme::Dark)).unwrap();
    });
    let (view, cx) = cx.add_window_view(|window, cx| Probe {
        number: NumberField::new(
            NumberWidth::FontSize,
            |owner: &mut Probe, text, _, _| owner.commits.push(text),
            |owner: &mut Probe, text, _, _| owner.edits.push(text),
            window,
            cx,
        ),
        other: TextField::new("", |_: &mut Probe, _, _, _| {}, window, cx),
        commits: Vec::new(),
        edits: Vec::new(),
    });
    // Test windows start inactive: GPUI suppresses Focus/Blur until activated.
    // Keep the same event/selection assertions as a real active native window.
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    view.update_in(cx, |owner, window, cx| {
        owner.number.set_text("14", window, cx);
        owner.number.field.focus(window, cx);
    });
    cx.run_until_parked();
    view.update_in(cx, |owner, _window, _cx| {
        assert!(
            owner.number.field.focused.get(),
            "the engine reported focus"
        );
    });
    cx.simulate_input("18");
    cx.run_until_parked();
    view.update_in(cx, |owner, _window, cx| {
        assert_eq!(owner.number.text(cx), "18");
        assert_eq!(owner.edits.last().map(String::as_str), Some("18"));
    });
    cx.simulate_keystrokes("enter");
    view.update_in(cx, |owner, _window, _cx| {
        assert_eq!(owner.commits, vec!["18"]);
    });
    view.update_in(cx, |owner, window, cx| owner.other.focus(window, cx));
    cx.run_until_parked();
    view.update_in(cx, |owner, _window, _cx| {
        assert_eq!(owner.commits, vec!["18", "18"]);
    });
    // An external preference update replaces the engine draft, then a new focus
    // selects the updated text rather than retaining a stale selection range.
    view.update_in(cx, |owner, window, cx| {
        owner.number.set_text("22", window, cx);
        owner.number.field.focus(window, cx);
    });
    cx.run_until_parked();
    cx.simulate_input("9");
    view.update_in(cx, |owner, _window, cx| {
        assert_eq!(owner.number.text(cx), "9");
    });
}

#[gpui::test]
fn both_number_profiles_keep_reference_width_in_a_real_narrow_parent(cx: &mut TestAppContext) {
    use gpui::{Bounds, Pixels};
    use std::cell::RefCell;
    use std::rc::Rc;
    struct BoundsProbe {
        field: NumberField,
        seen: Rc<RefCell<Vec<Bounds<Pixels>>>>,
    }
    impl Render for BoundsProbe {
        fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
            let seen = Rc::clone(&self.seen);
            div()
                .w(px(358.0))
                .flex()
                .flex_row()
                .on_children_prepainted(move |bounds, _, _| *seen.borrow_mut() = bounds)
                .child(self.field.element(&Tokens::of(crate::Scheme::Dark), cx))
        }
    }
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(crate::Scheme::Dark)).unwrap();
    });
    for (profile, width) in [
        (NumberWidth::FontSize, 64.0),
        (NumberWidth::ContentWidth, 80.0),
    ] {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let (view, cx) = cx.add_window_view({
            let seen = Rc::clone(&seen);
            move |window, cx| BoundsProbe {
                field: NumberField::new(
                    profile,
                    |_: &mut BoundsProbe, _, _, _| {},
                    |_: &mut BoundsProbe, _, _, _| {},
                    window,
                    cx,
                ),
                seen,
            }
        });
        view.update_in(cx, |owner, window, cx| {
            owner.field.set_text("4000", window, cx);
        });
        cx.run_until_parked();
        let bounds = seen.borrow().first().copied().unwrap();
        assert_eq!(bounds.size.width, px(width));
        assert!(bounds.size.height >= px(36.0));
        assert!(bounds.right() <= px(358.0));
    }
}

#[gpui::test]
fn clicking_either_rendered_numeric_profile_selects_its_previous_value(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(crate::Scheme::Dark)).unwrap();
    });
    for width in [NumberWidth::FontSize, NumberWidth::ContentWidth] {
        let (view, cx) = cx.add_window_view(move |window, cx| Probe {
            number: NumberField::new(
                width,
                |owner: &mut Probe, text, _, _| owner.commits.push(text),
                |owner: &mut Probe, text, _, _| owner.edits.push(text),
                window,
                cx,
            ),
            other: TextField::new("", |_: &mut Probe, _, _, _| {}, window, cx),
            commits: Vec::new(),
            edits: Vec::new(),
        });
        cx.update(|window, _| window.activate_window());
        cx.run_until_parked();
        for inset in [None, Some(-6.0), Some(6.0)] {
            view.update_in(cx, |owner, window, cx| {
                owner.number.set_text("14", window, cx);
                owner.other.focus(window, cx);
            });
            cx.run_until_parked();
            let position = view.update_in(cx, |owner, _window, cx| {
                let Kind::Line(input) = &owner.number.field.kind else {
                    panic!("number is a line");
                };
                let bounds = input.read(cx).input_bounds();
                assert!(bounds.size.width > px(0.0));
                match inset {
                    None => bounds.center(),
                    Some(offset) => {
                        let x = if offset < 0.0 {
                            px(f32::from(bounds.left()).mul_add(1.0, offset))
                        } else {
                            px(f32::from(bounds.right()).mul_add(1.0, offset))
                        };
                        let position = gpui::point(x, bounds.center().y);
                        assert!(!bounds.contains(&position), "probe is in shell padding");
                        position
                    }
                }
            });
            cx.simulate_click(position, gpui::Modifiers::none());
            cx.run_until_parked();
            view.update_in(cx, |owner, window, cx| {
                assert!(window.is_window_active());
                assert!(owner.number.field.focus_handle(cx).is_focused(window));
                assert!(owner.number.field.focused.get());
            });
            cx.simulate_input("18");
            cx.run_until_parked();
            view.update_in(cx, |owner, _window, cx| {
                assert_eq!(
                    owner.number.text(cx),
                    "18",
                    "text or padding focus selects the old value"
                );
            });
        }
    }
}
