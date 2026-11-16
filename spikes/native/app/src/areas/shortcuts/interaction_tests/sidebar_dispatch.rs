//! Actual keyboard dispatch consumes the one sidebar owner and W12 projection.
//! Passive chooser tabs carry no agent, terminal, file or backend authority.
use crate::{areas::session::Open, routes::Route};
use gpui::TestAppContext;
use vessel_app_view::{
    layout::{LayoutAction, LayoutKey, OpenIntent, Placement, Position, TabId, Target},
    prefs::Panel,
    shortcuts::Platform,
};

fn key() -> &'static str {
    match Platform::native() {
        Platform::Mac => "cmd-.",
        Platform::Other => "ctrl-.",
    }
}

#[gpui::test]
fn both_sidebar_key_uses_owner_intent_and_exits_focus_mode_idempotently(cx: &mut TestAppContext) {
    let (shell, cx) = super::setup(cx);
    cx.update(|window, _| window.activate_window());
    let (workspace, first, second): (LayoutKey, TabId, TabId) =
        shell.update_in(cx, |shell, window, cx| {
            shell.route = Route::Session;
            shell.areas.session.open = Some(Open {
                machine: "fixture".to_owned(),
                session: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
                title: "Workspace".to_owned(),
                agent: "fixture".to_owned(),
                structured: false,
                live: false,
            });
            let workspace = shell.layout_action_context().unwrap().workspace().clone();
            // The legacy draw bridge inserts an absent root agent on render.
            // Seed that metadata in the background so it cannot replace either
            // passive chooser selection while the actual Shell is drawn.
            let session = shell.areas.session.open.as_ref().unwrap().session;
            let _background = shell
                .layout_open_target(
                    &workspace,
                    Target::Agent(session),
                    OpenIntent::Background,
                    Placement::Focused,
                    window,
                    cx,
                )
                .unwrap();
            assert!(shell.run_layout_action(LayoutAction::NewTab, window, cx));
            let first = shell.layout_action_context().unwrap().selected().unwrap().0;
            assert!(shell.run_layout_action(LayoutAction::Split(Position::Right), window, cx));
            assert!(shell.run_layout_action(LayoutAction::NewTab, window, cx));
            let second = shell.layout_action_context().unwrap().selected().unwrap().0;
            assert!(shell.layout_panel_visible(&workspace, first, window));
            assert!(shell.layout_panel_visible(&workspace, second, window));
            assert!(shell.run_layout_action(LayoutAction::ToggleFocus, window, cx));
            assert!(!shell.layout_panel_visible(&workspace, first, window));
            assert!(shell.layout_panel_visible(&workspace, second, window));
            shell.prefs.settings.layout.sidebar = Panel::Hidden;
            shell.areas.workspace.code.open = true;
            window.blur(cx);
            (workspace, first, second)
        });
    cx.run_until_parked();
    for shown in [false, true] {
        cx.simulate_keystrokes(key());
        cx.run_until_parked();
        shell.update_in(cx, |shell, window, _| {
            assert_eq!(shell.sidebar_shown(), shown);
            assert_eq!(shell.areas.workspace.code.open, shown);
            assert!(shell.layout_panel_visible(&workspace, first, window));
            assert!(shell.layout_panel_visible(&workspace, second, window));
        });
    }
}

#[gpui::test]
fn both_sidebar_key_off_workspace_keeps_the_right_side_unchanged(cx: &mut TestAppContext) {
    let (shell, cx) = super::setup(cx);
    cx.update(|window, _| window.activate_window());
    shell.update_in(cx, |shell, window, cx| {
        shell.route = Route::Draft;
        shell.prefs.settings.layout.sidebar = Panel::Hidden;
        shell.areas.workspace.code.open = true;
        window.blur(cx);
    });
    cx.run_until_parked();
    cx.simulate_keystrokes(key());
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert!(shell.sidebar_shown());
        assert!(shell.areas.workspace.code.open);
    });
}
