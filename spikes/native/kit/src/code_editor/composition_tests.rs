//! Real marked text through the production `CodeEditor` constructor and registry.
use super::*;
use crate::TextField;
use gpui::{EntityInputHandler as _, Render, TestAppContext};

struct Plain;
impl LineSyntax for Plain {
    type State = ();
    fn language(&self) -> gpui::SharedString {
        "text".into()
    }
    fn line(&self, _text: &str, _state: ()) -> (Runs, ()) {
        (Vec::new(), ())
    }
}
struct Probe {
    editor: CodeEditor,
    other: TextField,
}
impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(crate::theme::Scheme::Dark);
        div()
            .size_full()
            .child(self.editor.element(&tokens, cx))
            .child(self.other.element(&tokens, false, cx))
    }
}
#[gpui::test]
fn code_editor_ime_uses_the_same_focused_composition_guard(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(crate::theme::Scheme::Dark)).unwrap();
    });
    let (probe, cx) = cx.add_window_view(|window, cx| Probe {
        editor: CodeEditor::new("", Plain, |_, _, _| {}, window, cx),
        other: TextField::new("", |_, _, _, _| {}, window, cx),
    });
    cx.run_until_parked();
    probe.update_in(cx, |probe, window, cx| {
        probe.editor.focus(window, cx);
        assert!(!TextField::focused_composing(window, cx));
        probe.editor.state.update(cx, |state, cx| {
            state.replace_and_mark_text_in_range(None, "하", None, window, cx);
        });
        assert!(TextField::focused_composing(window, cx));
        probe.other.focus(window, cx);
        assert!(
            !TextField::focused_composing(window, cx),
            "inactive editor does not own IME"
        );
        probe.editor.focus(window, cx);
        assert!(TextField::focused_composing(window, cx));
        probe
            .editor
            .state
            .update(cx, |state, cx| state.unmark_text(window, cx));
        assert!(
            !TextField::focused_composing(window, cx),
            "committed editor text"
        );
    });
}
