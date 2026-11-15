//! Actual App transactions refuse File disposal before publishing metadata.
use super::*;
use crate::areas::layout::{key, with_layout_at};
use crate::areas::session::Open;
use crate::shell::{Shell, language_tests};
use gpui::{AppContext as _, Context, TestAppContext, Window};
use vessel_app_view::layout::{
    AgentRole, CloseApproval, CloseScope, LayoutAction, LayoutKey, LayoutMode, Modified,
    OpenIntent, Placement, Position, RetargetRequest, TabId,
};
use vessel_proto::workspace::WorkspaceRoot;
use vessel_proto::{ProjectId, SessionId};

fn shell(window: &mut Window, cx: &mut Context<Shell>) -> Shell {
    let mut shell = Shell::without_io(window, cx);
    shell.route = crate::routes::Route::Session;
    shell.areas.session.open = Some(Open {
        machine: "isolated".to_owned(),
        session: SessionId::random(),
        title: "File guard fixture".to_owned(),
        agent: "fake".to_owned(),
        structured: true,
        live: true,
    });
    shell
}
fn file(path: &str) -> Target {
    Target::File {
        root: WorkspaceRoot::Project {
            project: ProjectId::random(),
        },
        path: path.parse().unwrap(),
    }
}
fn open(shell: &mut Shell, target: Target, window: &mut Window, cx: &mut Context<Shell>) -> TabId {
    shell
        .layout_open_target(
            &key(shell).unwrap(),
            target,
            OpenIntent::Reveal,
            Placement::Focused,
            window,
            cx,
        )
        .unwrap()
}
fn move_file(shell: &mut Shell, key: &LayoutKey, tab: TabId, cx: &mut Context<Shell>) {
    shell
        .update_layout(
            key,
            |layout| {
                let pane = layout.split(
                    layout.focused_pane(),
                    Position::Right,
                    LayoutMode::SplitPanes,
                )?;
                layout.drop_tab(tab, pane, Position::Center, LayoutMode::SplitPanes)?;
                layout.set_modified(tab, Modified::Dirty)
            },
            cx,
        )
        .unwrap();
}

#[gpui::test]
fn cached_clean_file_refuses_generic_retarget_and_close_fallback(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(shell);
    shell.update_in(cx, |shell, window, cx| {
        let key = key(shell).unwrap();
        let target = file("original.rs");
        let tab = open(shell, target.clone(), window, cx);
        assert_eq!(
            shell.update_layout(
                &key,
                |layout| layout.retarget(RetargetRequest {
                    tab,
                    target: Target::NewTab,
                    intent: OpenIntent::Reveal,
                    role: AgentRole::Unknown,
                }),
                cx
            ),
            Err(LayoutError::UnsupportedHost)
        );
        assert!(shell.run_layout_action(LayoutAction::Close(CloseScope::Current), window, cx));
        assert!(shell.layout_tab_matches(&key, tab, &target));
        assert_eq!(
            with_layout_at(shell, &key, |layout| Ok(layout
                .tab(tab)
                .unwrap()
                .modified())),
            Ok(Modified::Clean)
        );
    });
}

#[gpui::test]
fn earlier_compact_reveal_and_direct_dismissal_preserve_later_clean_files(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(shell);
    cx.simulate_resize(gpui::size(gpui::px(390.0), gpui::px(844.0)));
    shell.update_in(cx, |shell, window, cx| {
        let key = key(shell).unwrap();
        let a = file("a.rs");
        let first = open(shell, a.clone(), window, cx);
        let b = file("b.rs");
        let second = open(shell, b.clone(), window, cx);
        let before = shell.areas.layout.book.borrow().clone();
        assert_eq!(
            shell.layout_open_target(
                &key,
                a.clone(),
                OpenIntent::Reveal,
                Placement::Focused,
                window,
                cx
            ),
            Err(LayoutError::UnsupportedHost)
        );
        assert_eq!(*shell.areas.layout.book.borrow(), before);
        shell.layout_dismiss_sheet(&key, second, cx);
        assert_eq!(*shell.areas.layout.book.borrow(), before);
        assert!(shell.layout_tab_matches(&key, first, &a));
        assert!(shell.layout_tab_matches(&key, second, &b));
    });
}

#[gpui::test]
fn moving_and_modified_updates_work_but_remove_then_reopen_cannot_replace_the_old_tab(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(shell);
    cx.simulate_resize(gpui::size(gpui::px(1280.0), gpui::px(900.0)));
    shell.update_in(cx, |shell, window, cx| {
        let key = key(shell).unwrap();
        let target = file("retained.rs");
        let tab = open(shell, target.clone(), window, cx);
        move_file(shell, &key, tab, cx);
        shell
            .update_layout(&key, |layout| layout.set_modified(tab, Modified::Clean), cx)
            .unwrap();
        let before = shell.areas.layout.book.borrow().clone();
        assert_eq!(
            shell.update_layout(
                &key,
                |layout| {
                    let plan = layout.request_close(tab)?;
                    layout.close_confirmed(&plan, CloseApproval::Local)?;
                    layout.open(
                        target.clone(),
                        OpenIntent::Reveal,
                        Placement::Focused,
                        AgentRole::Unknown,
                        LayoutMode::SplitPanes,
                    )
                },
                cx
            ),
            Err(LayoutError::UnsupportedHost)
        );
        assert_eq!(*shell.areas.layout.book.borrow(), before);
        let plan = with_layout_at(shell, &key, |layout| layout.request_close(tab)).unwrap();
        // Completion mechanism only; actual owner admission is not supplied here.
        shell.layout_close_settled(&key, &plan, Ok(()), cx);
        assert!(!shell.layout_tab_exists(&key, tab));
    });
}
