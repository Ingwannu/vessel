//! Query-only root action goes through the real dispatcher and palette frame.
use super::*;

#[gpui::test]
fn query_sidebar_action_uses_the_existing_keyboard_owner(cx: &mut TestAppContext) {
    let (view, cx) = harness(cx);
    let (before, label) = update(&view, cx, |shell, window, cx| {
        crate::areas::shortcuts::listen(shell, window, cx);
        let entries = root::build(shell, &shell.palette_all());
        let entry = entries
            .iter()
            .find(|entry| entry.id.as_str() == "toggle-left-sidebar")
            .unwrap();
        assert_eq!(
            entry.visibility,
            vessel_app_view::palette::registration::Visibility::Query
        );
        let before = (shell.sidebar_open, shell.sidebar_shown);
        let label = vessel_app_view::shortcuts::action_label(
            vessel_app_view::shortcuts::Action::ToggleSidebar,
            shell.words,
        )
        .to_owned();
        (before, label)
    });
    ready(cx);
    cx.simulate_input(&label);
    ready(cx);
    cx.simulate_keystrokes("enter");
    ready(cx);
    update(&view, cx, |shell, _, _| {
        assert!(!shell.areas.palette.is_open());
        assert_ne!((shell.sidebar_open, shell.sidebar_shown), before);
    });
}
