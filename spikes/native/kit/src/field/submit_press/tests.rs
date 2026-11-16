//! Real engine callbacks preserve modifiers only for their synchronous owner.
use super::*;
use crate::{Scheme, Tokens, field::SendOn};
use gpui::{Context, IntoElement, Render, TestAppContext, Window};

struct Probe {
    field: TextField,
    observed: Vec<Option<SubmitPress>>,
}
impl Probe {
    fn submitted(&mut self, _: String, _: &mut Window, _: &mut Context<'_, Self>) {
        self.observed.push(self.field.submit_press());
    }
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.field.element(&Tokens::of(Scheme::Dark), false, cx)
    }
}

#[gpui::test]
fn engine_modifiers_are_scoped_to_the_existing_callback(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    let (probe, window) = cx.add_window_view(|window, cx| {
        let field = TextField::composer("Message", Probe::submitted, window, cx);
        field.focus(window, cx);
        Probe {
            field,
            observed: Vec::new(),
        }
    });
    window.run_until_parked();
    window.simulate_input("text");
    window.simulate_keystrokes("enter");
    let modified = probe.read_with(window, |_, cx| {
        if cx.platform_info().is_mac() {
            "cmd-enter"
        } else {
            "ctrl-enter"
        }
    });
    window.simulate_keystrokes(modified);
    window.run_until_parked();
    probe.read_with(window, |probe, cx| {
        assert_eq!(
            probe.observed,
            [Some(SubmitPress::Enter), Some(SubmitPress::ModifiedEnter)]
        );
        assert_eq!(probe.field.submit_press(), None);
        assert_eq!(probe.field.text(cx), "text");
    });
    window
        .update(|_, cx| probe.read_with(cx, |probe, _| probe.field.set_send_on(SendOn::ModEnter)));
    window.simulate_keystrokes("enter");
    window.run_until_parked();
    probe.read_with(window, |probe, cx| {
        assert_eq!(
            probe.observed.len(),
            2,
            "newline must not fabricate submit metadata"
        );
        assert_eq!(probe.field.submit_press(), None);
        assert_eq!(probe.field.text(cx), "text\n");
    });
}

#[test]
fn nested_dispatch_restores_the_outer_press_then_clears_it() {
    let cell = Rc::new(Cell::new(None));
    {
        let _outer = Scope::enter(&cell, false);
        {
            let _inner = Scope::enter(&cell, true);
            assert_eq!(cell.get(), Some(SubmitPress::ModifiedEnter));
        }
        assert_eq!(cell.get(), Some(SubmitPress::Enter));
    }
    assert_eq!(cell.get(), None);
}
