//! Real GPUI/engine selection, clipboard and marked-text paths, not a second editor.

use std::ops::Range;

use gpui::prelude::*;
use gpui::{
    App, ClipboardItem, Context, Entity, EntityInputHandler as _, Render, TestAppContext,
    VisualTestContext, Window, div,
};
use gpui_base::input::{InputBaseState, InputModeKind};

use super::{Kind, TextField};
use crate::field::Editability;
use crate::{Scheme, Tokens};

#[derive(Clone, Copy)]
enum Flavor {
    Line,
    Composer,
    Form,
}
const FLAVORS: [Flavor; 3] = [Flavor::Line, Flavor::Composer, Flavor::Form];

struct Probe {
    field: TextField,
    flavor: Flavor,
    submitted: usize,
}

impl Probe {
    fn new(flavor: Flavor, window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let submit: fn(&mut Self, String, &mut Window, &mut Context<'_, Self>) =
            |owner, _, _, _| owner.submitted = owner.submitted.saturating_add(1);
        let field = match flavor {
            Flavor::Line => TextField::new("Name", submit, window, cx),
            Flavor::Composer => TextField::composer("Message", submit, window, cx),
            Flavor::Form => TextField::form_multiline("Body", submit, window, cx),
        };
        Self {
            field,
            flavor,
            submitted: 0,
        }
    }
}

impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let field = match self.flavor {
            Flavor::Line => self.field.rename_element(&tokens, cx).into_any_element(),
            Flavor::Composer | Flavor::Form => self.field.element(&tokens, false, cx),
        };
        div().child(field)
    }
}

fn open(cx: &mut TestAppContext, flavor: Flavor) -> (Entity<Probe>, &mut VisualTestContext) {
    let (probe, cx) = cx.add_window_view(move |window, cx| Probe::new(flavor, window, cx));
    cx.update(|window, _cx| window.activate_window());
    cx.run_until_parked();
    (probe, cx)
}

fn value(probe: &Entity<Probe>, cx: &VisualTestContext) -> String {
    probe.read_with(cx, |probe, cx| probe.field.text(cx))
}

fn reopen(probe: &Entity<Probe>, text: &str, cx: &mut VisualTestContext) {
    probe.update_in(cx, |probe, window, cx| {
        probe.field.set_editability(Editability::Editable, cx);
        probe.field.set_text(text, window, cx);
        probe.field.focus(window, cx);
        probe.field.select_all(window, cx);
    });
    cx.run_until_parked();
}

#[gpui::test]
fn opening_selects_the_whole_unicode_value_and_first_input_replaces_it(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    for flavor in FLAVORS {
        let (probe, cx) = open(cx, flavor);
        for original in ["laptop", "한글-machine", "😀고"] {
            reopen(&probe, original, cx);
            probe.read_with(cx, |probe, cx| {
                assert_eq!(probe.field.selection(cx), 0..original.len());
            });
            // A rendered frame must not reset the initial selection to a caret.
            cx.run_until_parked();
            cx.simulate_input("replacement");
            assert_eq!(value(&probe, cx), "replacement");
            probe.read_with(cx, |probe, _| assert_eq!(probe.submitted, 0));
        }
    }
}

#[derive(Clone, Copy)]
enum UserEdit {
    Commit,
    Mark,
}

fn edit_engine<M: InputModeKind>(
    state: &mut InputBaseState<M>,
    edit: UserEdit,
    text: &str,
    window: &mut Window,
    cx: &mut Context<'_, InputBaseState<M>>,
) -> Option<Range<usize>> {
    match edit {
        UserEdit::Commit => state.replace_text_in_range(None, text, window, cx),
        UserEdit::Mark => state.replace_and_mark_text_in_range(None, text, Some(1..1), window, cx),
    }
    state.marked_text_range(window, cx)
}

fn ime(
    field: &TextField,
    edit: UserEdit,
    text: &str,
    window: &mut Window,
    cx: &mut App,
) -> Option<Range<usize>> {
    match &field.kind {
        Kind::Line(state) => {
            state.update(cx, |state, cx| edit_engine(state, edit, text, window, cx))
        }
        Kind::Lines(state, _) => {
            state.update(cx, |state, cx| edit_engine(state, edit, text, window, cx))
        }
    }
}

#[gpui::test]
fn readonly_refuses_typing_paste_and_both_ime_paths_then_reenables(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    for flavor in FLAVORS {
        let (probe, cx) = open(cx, flavor);
        reopen(&probe, "laptop", cx);
        probe.update_in(cx, |probe, _, cx| {
            probe.field.set_editability(Editability::ReadOnly, cx);
        });
        cx.simulate_input("blocked");
        assert_eq!(value(&probe, cx), "laptop");
        cx.update(|_, cx| cx.write_to_clipboard(ClipboardItem::new_string("pasted".into())));
        let paste = cx.read(|cx| {
            if cx.platform_info().is_mac() {
                "cmd-v"
            } else {
                "ctrl-v"
            }
        });
        cx.simulate_keystrokes(paste);
        assert_eq!(value(&probe, cx), "laptop");
        probe.update_in(cx, |probe, window, cx| {
            assert_eq!(
                ime(&probe.field, UserEdit::Commit, "commit", window, cx),
                None
            );
            assert_eq!(probe.field.text(cx), "laptop");
            assert_eq!(ime(&probe.field, UserEdit::Mark, "ㅎ", window, cx), None);
            assert_eq!(probe.field.text(cx), "laptop");
            assert!(!TextField::focused_composing(window, cx));
            probe.field.set_editability(Editability::Editable, cx);
        });
        cx.simulate_keystrokes(paste);
        assert_eq!(value(&probe, cx), "pasted");
        probe.update_in(cx, |probe, window, cx| {
            probe.field.select_all(window, cx);
            assert_eq!(
                ime(&probe.field, UserEdit::Mark, "ㅎ", window, cx),
                Some(0..1)
            );
            assert!(
                TextField::focused_composing(window, cx),
                "still in the existing registry"
            );
            assert_eq!(ime(&probe.field, UserEdit::Commit, "한", window, cx), None);
            assert_eq!(probe.field.text(cx), "한");
        });
    }
}

#[gpui::test]
fn reenable_keeps_the_failed_draft_and_reopening_resets_it(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    let (probe, cx) = open(cx, Flavor::Line);
    reopen(&probe, "laptop", cx);
    cx.simulate_input("draft");
    probe.update_in(cx, |probe, _, cx| {
        probe.field.set_editability(Editability::ReadOnly, cx);
        probe.field.set_editability(Editability::Editable, cx);
    });
    assert_eq!(value(&probe, cx), "draft");
    cx.simulate_input("-corrected");
    assert_eq!(value(&probe, cx), "draft-corrected");
    reopen(&probe, "laptop", cx);
    cx.simulate_input("fresh");
    assert_eq!(value(&probe, cx), "fresh");
}
