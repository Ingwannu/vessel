//! The rename dialog (Paseo's `AdaptiveRenameModal`): the title prefilled,
//! Rename only for another title the node would take, the reason for one it
//! would not, and Cancel keeping the title.
use super::*;
use crate::shell::language_tests;
use gpui::{AppContext as _, Render, TestAppContext};
use vessel_app_kit::ui::{Entity, IntoElement};

struct Probe(Entity<Shell>);

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = vessel_app_kit::tokens(cx);
        self.0
            .update(cx, |shell, cx| div().children(rename_dialog(shell, &tokens, window, cx)))
    }
}

fn typed(probe: &Entity<Probe>, cx: &mut gpui::VisualTestContext, text: &'static str) {
    probe.update_in(cx, |probe, window, cx| {
        probe.0.update(cx, |shell, cx| {
            if let Some(rename) = shell.areas.conversation.rename.as_ref() {
                rename.field.set_text(text, window, cx);
            }
        });
    });
}

fn state(probe: &Entity<Probe>, cx: &mut gpui::VisualTestContext) -> Option<(String, bool, bool)> {
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        shell.areas.conversation.rename.as_ref().map(|rename| {
            let text = rename.field.text(cx);
            let save = savable(rename, &text);
            (text, save, rename.invalid.is_some())
        })
    })
}

#[gpui::test]
fn the_dialog_opens_prefilled_and_saves_only_another_valid_title(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| Probe(cx.new(|cx| Shell::without_io(window, cx))));
    probe.update_in(cx, |probe, window, cx| {
        probe.0.update(cx, |shell, cx| shell.start_rename("Fix the cart", window, cx));
    });
    cx.run_until_parked();
    assert_eq!(
        state(&probe, cx),
        Some(("Fix the cart".to_owned(), false, false)),
        "prefilled; Rename waits for another title"
    );
    // A title the node would refuse: the reason, and the dialog stays.
    typed(&probe, cx, "");
    probe.update_in(cx, |probe, window, cx| {
        probe.0.update(cx, |shell, cx| shell.rename_pressed(window, cx));
    });
    assert_eq!(state(&probe, cx).map(|(_, save, invalid)| (save, invalid)), Some((false, true)));
    typed(&probe, cx, "Fix the basket");
    assert_eq!(state(&probe, cx).map(|(_, save, _)| save), Some(true));
    // Cancel: the title stays and the dialog closes.
    probe.update_in(cx, |probe, window, cx| {
        probe.0.update(cx, |shell, cx| shell.cancel_rename(window, cx));
    });
    assert_eq!(state(&probe, cx), None);
}
