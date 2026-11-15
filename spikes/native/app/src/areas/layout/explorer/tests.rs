//! Real layout metadata reaches the host once, without resource or tab fabrication.
use super::*;
use crate::areas::session::Open;
use crate::shell::language_tests;
use gpui::TestAppContext;
use vessel_app_view::layout::{
    AgentRole, LayoutKey, LayoutMode, OpenIntent, Placement, TabId, Target,
};
use vessel_proto::SessionId;

fn shell(window: &mut gpui::Window, cx: &mut gpui::Context<'_, Shell>) -> Shell {
    let mut shell = Shell::without_io(window, cx);
    shell.route = crate::routes::Route::Session;
    shell.areas.session.open = Some(Open {
        machine: "isolated".to_owned(),
        session: SessionId::random(),
        title: "Explorer fixture".to_owned(),
        agent: "fake".to_owned(),
        structured: true,
        live: true,
    });
    shell
}

fn open(
    shell: &mut Shell,
    key: &LayoutKey,
    target: Target,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<'_, Shell>,
) -> TabId {
    shell
        .layout_open_target(
            key,
            target,
            OpenIntent::Reveal,
            Placement::Explorer,
            window,
            cx,
        )
        .unwrap()
}

#[gpui::test]
fn explorer_projection_uses_actual_tab_and_releases_the_book_before_render(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(shell);
    shell.update_in(cx, |shell, window, cx| {
        let key = super::super::key(shell).unwrap();
        let tab = open(shell, &key, Target::Files, window, cx);
        let rendered = shell.layout_with_explorer(PanelFocus::Background, |panel| {
            assert_eq!(panel.workspace, &key);
            assert_eq!(panel.pane, PaneId::EXPLORER);
            assert_eq!(panel.tab, tab);
            assert_eq!(panel.target, &Target::Files);
            assert_eq!(panel.focus, PanelFocus::Background);
            assert!(shell.areas.layout.book.try_borrow_mut().is_ok());
            panel.tab
        });
        assert_eq!(rendered, Some(tab));
        // This staged projection does not make an unmounted Explorer visible.
        assert!(!shell.layout_panel_visible(&key, tab, window));
    });
}

#[gpui::test]
fn hidden_explorer_and_covering_sheet_keep_the_tab_without_rendering(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(shell);
    shell.update_in(cx, |shell, window, cx| {
        let key = super::super::key(shell).unwrap();
        let tab = open(shell, &key, Target::Changes, window, cx);
        shell
            .update_layout(
                &key,
                |layout| {
                    layout.show_explorer(ExplorerVisibility::Hidden);
                    Ok(())
                },
                cx,
            )
            .unwrap();
        assert_eq!(
            shell.layout_with_explorer(PanelFocus::Focused, |_| true),
            None
        );
        assert!(shell.layout_tab_matches(&key, tab, &Target::Changes));
        shell
            .update_layout(
                &key,
                |layout| {
                    layout.show_explorer(ExplorerVisibility::Shown);
                    layout
                        .open(
                            Target::Agent(SessionId::random()),
                            OpenIntent::New,
                            Placement::Focused,
                            AgentRole::Child,
                            LayoutMode::Compact,
                        )
                        .map(|_| ())
                },
                cx,
            )
            .unwrap();
        assert_eq!(
            shell.layout_with_explorer(PanelFocus::Focused, |_| true),
            None
        );
        assert!(shell.layout_tab_matches(&key, tab, &Target::Changes));
    });
}

#[gpui::test]
fn cold_absent_workspace_does_not_allocate_an_explorer_or_call_its_body(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(shell);
    shell.update_in(cx, |shell, _window, _cx| {
        assert!(shell.areas.layout.book.borrow().is_none());
        assert_eq!(
            shell.layout_with_explorer(PanelFocus::Focused, |_| true),
            None
        );
        assert!(
            shell
                .areas
                .layout
                .book
                .borrow()
                .as_ref()
                .unwrap()
                .is_empty()
        );
    });
}
