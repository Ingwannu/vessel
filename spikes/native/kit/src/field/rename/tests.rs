//! Exercise the real input engine's keyboard, paste and IME entry points.
use super::*;
use gpui::{ClipboardItem, Context, EntityInputHandler as _, IntoElement, Render, TestAppContext};
struct Probe {
    field: TextField,
}
impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.field
            .rename_element(&Tokens::of(crate::Scheme::Dark), cx)
    }
}
#[gpui::test]
fn a_pending_rename_rejects_typing_paste_and_marked_text_then_edits_again(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(crate::Scheme::Dark)).unwrap();
    });
    let (view, cx) = cx.add_window_view(|window, cx| {
        let field = TextField::new("", |_: &mut Probe, _, _, _| {}, window, cx);
        field.set_text("laptop", window, cx);
        field.focus(window, cx);
        field.select_all(window, cx);
        field.set_editability(Editability::ReadOnly, cx);
        Probe { field }
    });
    cx.run_until_parked();
    cx.simulate_input("typed");
    cx.update(|_window, cx| cx.write_to_clipboard(ClipboardItem::new_string("pasted".to_owned())));
    cx.simulate_keystrokes(if cfg!(target_os = "macos") {
        "cmd-v"
    } else {
        "ctrl-v"
    });
    view.update_in(cx, |view, window, cx| {
        let Kind::Line(state) = &view.field.kind else {
            panic!("rename is a line")
        };
        state.update(cx, |state, cx| {
            state.replace_and_mark_text_in_range(None, "ㅎ", None, window, cx);
            state.replace_text_in_range(None, "한", window, cx);
            assert!(!state.is_editable());
        });
        assert_eq!(view.field.text(cx), "laptop");
        view.field.set_editability(Editability::Editable, cx);
        view.field.select_all(window, cx);
    });
    cx.simulate_input("work");
    view.update_in(cx, |view, _window, cx| {
        assert_eq!(view.field.text(cx), "work");
    });
}

#[gpui::test]
fn rename_padding_focuses_the_same_engine_and_readonly_still_refuses_input(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(crate::Scheme::Dark)).unwrap();
    });
    let (view, cx) = cx.add_window_view(|window, cx| {
        let field = TextField::new("", |_: &mut Probe, _, _, _| {}, window, cx);
        field.set_text("laptop", window, cx);
        field.select_all(window, cx);
        Probe { field }
    });
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    for readonly in [false, true] {
        view.update_in(cx, |view, _window, cx| {
            view.field.set_editability(
                if readonly {
                    Editability::ReadOnly
                } else {
                    Editability::Editable
                },
                cx,
            );
        });
        cx.update(Window::blur);
        cx.run_until_parked();
        let padding = view.update_in(cx, |view, _window, cx| {
            let Kind::Line(input) = &view.field.kind else {
                panic!("rename is a line");
            };
            let bounds = input.read(cx).input_bounds();
            let padding = gpui::point(
                px(f32::from(bounds.left()).mul_add(1.0, -6.0)),
                bounds.center().y,
            );
            assert!(!bounds.contains(&padding));
            padding
        });
        cx.simulate_click(padding, gpui::Modifiers::none());
        cx.run_until_parked();
        view.update_in(cx, |view, window, cx| {
            assert!(view.field.focus_handle(cx).is_focused(window));
            assert!(view.field.focused.get());
        });
        cx.simulate_input(if readonly { "blocked" } else { "work" });
        cx.run_until_parked();
        view.update_in(cx, |view, _window, cx| {
            assert_eq!(view.field.text(cx), "work");
        });
    }
}
