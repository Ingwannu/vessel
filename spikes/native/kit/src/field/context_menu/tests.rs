//! Real fields enter the existing deferred input hook; no test-only menu engine.

use super::*;
use crate::field::Editability;
use crate::{Scheme, Tokens};
use gpui::prelude::*;
use gpui::{Context, Modifiers, MouseButton, Render, TestAppContext, div, point, px};
use std::cell::RefCell;

#[derive(Clone, Copy)]
enum Flavor {
    Line,
    Composer,
    Form,
}

struct Field {
    input: TextField,
    captured: Rc<RefCell<Vec<Request>>>,
}

impl Field {
    fn new(flavor: Flavor, opt_in: bool, window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let submit: fn(&mut Self, String, &mut Window, &mut Context<'_, Self>) = |_, _, _, _| {};
        let input = match flavor {
            Flavor::Line => TextField::new("Name", submit, window, cx),
            Flavor::Composer => TextField::composer("Message", submit, window, cx),
            Flavor::Form => TextField::form_multiline("Body", submit, window, cx),
        };
        let captured = Rc::new(RefCell::new(Vec::new()));
        if opt_in {
            let received = Rc::clone(&captured);
            input.set_context_menu_handler(
                Rc::new(move |request, _, _| {
                    received.borrow_mut().push(request);
                }),
                cx,
            );
        }
        input.set_text("alpha beta", window, cx);
        input.focus(window, cx);
        input.select_all(window, cx);
        Self { input, captured }
    }
}

impl Render for Field {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        div().w(px(360.0)).h(px(150.0)).child(self.input.element(
            &Tokens::of(Scheme::Dark),
            false,
            cx,
        ))
    }
}

#[gpui::test]
fn actual_line_and_multiline_right_clicks_deliver_current_permissions(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    for flavor in [Flavor::Line, Flavor::Composer, Flavor::Form] {
        let (field, cx) =
            cx.add_window_view(move |window, cx| Field::new(flavor, true, window, cx));
        cx.run_until_parked();
        cx.simulate_mouse_down(
            point(px(20.0), px(16.0)),
            MouseButton::Right,
            Modifiers::default(),
        );
        cx.simulate_mouse_up(
            point(px(20.0), px(16.0)),
            MouseButton::Right,
            Modifiers::default(),
        );
        cx.run_until_parked();
        field.update_in(cx, |field, window, cx| {
            let captured = field.captured.borrow();
            assert_eq!(captured.len(), 1);
            let request = &captured[0];
            assert!(request.current(window, cx));
            assert_eq!(
                request.facts(cx),
                Some(Facts {
                    copy: CopyPermission::Available,
                    paste: PastePermission::Available,
                    editing: Editing::Editable,
                })
            );
            field.input.set_editability(Editability::ReadOnly, cx);
            assert_eq!(
                request.facts(cx),
                Some(Facts {
                    copy: CopyPermission::Available,
                    paste: PastePermission::Unavailable,
                    editing: Editing::ReadOnly,
                })
            );
            let replacement = cx.focus_handle();
            replacement.focus(window, cx);
            assert!(!request.current(window, cx));
            request.retain_selection(window, cx);
            let highlighted = match &field.input.kind {
                Kind::Line(input) => input.read(cx).has_selection_focus(window, cx),
                Kind::Lines(input, _) => input.read(cx).has_selection_focus(window, cx),
            };
            assert!(
                highlighted,
                "the existing editor keeps its selection for the popup"
            );
            request.release_selection(cx);
            let highlighted = match &field.input.kind {
                Kind::Line(input) => input.read(cx).has_selection_focus(window, cx),
                Kind::Lines(input, _) => input.read(cx).has_selection_focus(window, cx),
            };
            assert!(
                !highlighted,
                "a closed popup must not retain another editor's highlight"
            );
            assert_eq!(field.input.text(cx), "alpha beta");
        });
    }
}

#[gpui::test]
fn a_retained_request_rechecks_masking_and_never_retains_its_input(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    let (field, cx) = cx.add_window_view(|window, cx| Field::new(Flavor::Line, true, window, cx));
    cx.run_until_parked();
    cx.simulate_mouse_down(
        point(px(20.0), px(16.0)),
        MouseButton::Right,
        Modifiers::default(),
    );
    cx.simulate_mouse_up(
        point(px(20.0), px(16.0)),
        MouseButton::Right,
        Modifiers::default(),
    );
    cx.run_until_parked();
    let request = field.update_in(cx, |field, window, cx| {
        let request = field.captured.borrow()[0].clone();
        let Kind::Line(input) = &field.input.kind else {
            panic!("line fixture");
        };
        input.update(cx, |input, cx| input.toggle_masked(window, cx));
        assert_eq!(request.facts(cx).unwrap().copy, CopyPermission::Unavailable);
        request
    });
    cx.update(|window, cx| {
        let _empty = window.replace_root(cx, |_, _| Empty);
    });
    drop(field);
    cx.run_until_parked();
    cx.update(|window, cx| {
        assert!(request.facts(cx).is_none());
        assert!(!request.current(window, cx));
    });
}

struct Empty;
impl Render for Empty {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        div()
    }
}

#[gpui::test]
fn unconfigured_fields_keep_their_original_hook(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    let (field, cx) = cx.add_window_view(|window, cx| Field::new(Flavor::Line, false, window, cx));
    cx.run_until_parked();
    cx.simulate_mouse_down(
        point(px(20.0), px(16.0)),
        MouseButton::Right,
        Modifiers::default(),
    );
    cx.simulate_mouse_up(
        point(px(20.0), px(16.0)),
        MouseButton::Right,
        Modifiers::default(),
    );
    cx.run_until_parked();
    field.read_with(cx, |field, _| assert!(field.captured.borrow().is_empty()));
}

#[test]
fn disabled_and_masked_facts_cannot_advertise_copy_or_paste() {
    let disabled = Facts::from(
        InputContextMenuCapabilities::new()
            .selection(true)
            .disabled(true),
    );
    assert_eq!(disabled.copy, CopyPermission::Unavailable);
    assert_eq!(disabled.paste, PastePermission::Unavailable);
    assert_eq!(disabled.editing, Editing::Disabled);
    let masked = Facts::from(
        InputContextMenuCapabilities::new()
            .selection(true)
            .masked(true),
    );
    assert_eq!(masked.copy, CopyPermission::Unavailable);
    assert_eq!(masked.paste, PastePermission::Available);
}
