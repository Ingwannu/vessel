//! The actual input subscription preserves its captured opening on Enter.
use super::*;
use gpui::{Render, TestAppContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Opening(u64);

struct Probe {
    field: TextField,
    selected: Opening,
    submitted: Option<(Opening, String)>,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        div().child(
            self.field
                .input(&Tokens::of(crate::theme::Scheme::Dark), cx),
        )
    }
}

#[gpui::test]
fn enter_uses_the_original_capture_after_owner_selection_changes(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    let original = Opening(1);
    let (probe, cx) = cx.add_window_view(move |window, cx| Probe {
        field: TextField::new(
            "Name",
            move |owner: &mut Probe, text, _, _| {
                owner.submitted = Some((original, text));
            },
            window,
            cx,
        ),
        selected: original,
        submitted: None,
    });
    probe.update_in(cx, |probe, window, cx| {
        window.activate_window();
        probe.selected = Opening(2);
        probe.field.focus(window, cx);
    });
    cx.run_until_parked();
    cx.simulate_input("captured.txt");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    cx.update(|_, cx| {
        probe.read_with(cx, |probe, _| {
            assert_eq!(probe.selected, Opening(2));
            assert_eq!(probe.submitted, Some((original, "captured.txt".to_owned())));
        })
    });
}
