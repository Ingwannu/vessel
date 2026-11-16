//! Same production editor: A -> X -> A retains synchronous edit evidence.
use super::*;
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
struct Probe(CodeEditor);
impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.0.element(&Tokens::of(crate::theme::Scheme::Dark), cx)
    }
}

#[gpui::test]
fn existing_revision_distinguishes_equal_text_after_accepted_edits(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(crate::theme::Scheme::Dark)).unwrap();
    });
    let (probe, cx) = cx
        .add_window_view(|window, cx| Probe(CodeEditor::new("A", Plain, |_, _, _| {}, window, cx)));
    probe.update_in(cx, |probe, window, cx| {
        let before = probe.0.revision(cx);
        probe.0.state.update(cx, |state, cx| {
            state.replace_text_in_range(Some(0..1), "X", window, cx);
        });
        let changed = probe.0.revision(cx);
        assert_ne!(before, changed);
        probe.0.state.update(cx, |state, cx| {
            state.replace_text_in_range(Some(0..1), "A", window, cx);
        });
        let returned = probe.0.revision(cx);
        assert_ne!(changed, returned);
        assert_ne!(before, returned);
        assert_eq!(probe.0.text(cx), "A");
        assert_eq!(probe.0.revision(cx), returned);
    });
}
