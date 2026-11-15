//! Real book transaction hooks, not a claim about a feature's pending UI state.
use super::*;
use crate::shell::language_tests;
use gpui::TestAppContext;
use vessel_app_view::layout::{
    AgentRole, Modified, OpenIntent, Placement, Position, RetargetRequest,
};
use vessel_proto::{NodeId, WorkspaceId};

fn key() -> LayoutKey {
    LayoutKey::Workspace {
        node: NodeId::random(),
        workspace: WorkspaceId::random(),
    }
}
fn observed(shell: &mut Shell, notice: &Notice, _cx: &mut Context<'_, Shell>) {
    // This read would panic if dispatch retained the mutable book borrow.
    let book = shell.areas.layout.book.borrow();
    if matches!(notice.reason(), Reason::Closed) {
        assert!(
            book.as_ref()
                .and_then(|book| book.get(notice.workspace()))
                .and_then(|layout| layout.tab(notice.tab()))
                .is_none()
        );
    }
    shell.areas.layout.menu_tab = Some((notice.workspace().clone(), notice.tab()));
    shell.notice = Some(format!("{:?}:{:?}", notice.reason(), notice.target()));
}
fn forbidden_replacement(_shell: &mut Shell, _notice: &Notice, _cx: &mut Context<'_, Shell>) {
    panic!("a duplicate registration replaced the original owner");
}
fn retarget(
    shell: &mut Shell,
    key: &LayoutKey,
    tab: TabId,
    target: Target,
    cx: &mut Context<'_, Shell>,
) {
    let _tab = shell
        .update_layout(
            key,
            |layout| {
                layout.retarget(RetargetRequest {
                    tab,
                    target,
                    intent: OpenIntent::Reveal,
                    role: AgentRole::Unknown,
                })
            },
            cx,
        )
        .unwrap();
}

#[gpui::test]
fn two_committed_retargets_notify_before_any_poll_and_duplicate_registration_refuses(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        shell
            .layout_register_retirement(Owner::Scripts, observed)
            .unwrap();
        shell
            .layout_register_retirement(Owner::Git, observed)
            .unwrap();
        assert_eq!(
            shell.layout_register_retirement(Owner::Scripts, forbidden_replacement),
            Err(LayoutError::InvalidState)
        );
        let key = key();
        let tab = shell
            .layout_open_target(
                &key,
                Target::Scripts,
                OpenIntent::Reveal,
                Placement::Focused,
                window,
                cx,
            )
            .unwrap();
        retarget(shell, &key, tab, Target::WorkingDiff, cx);
        assert_eq!(shell.notice.as_deref(), Some("Retargeted:Scripts"));
        // No renderer/poll ran between either committed mutation.
        retarget(shell, &key, tab, Target::Scripts, cx);
        assert_eq!(shell.notice.as_deref(), Some("Retargeted:WorkingDiff"));
    });
}

#[gpui::test]
fn failed_batch_and_pane_moves_do_not_retire_the_owner(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        shell
            .layout_register_retirement(Owner::Scripts, observed)
            .unwrap();
        let key = key();
        let tab = shell
            .layout_open_target(
                &key,
                Target::Scripts,
                OpenIntent::Reveal,
                Placement::Focused,
                window,
                cx,
            )
            .unwrap();
        let result: Result<(), LayoutError> = shell.update_layout(
            &key,
            |layout| {
                let _tab = layout.retarget(RetargetRequest {
                    tab,
                    target: Target::WorkingDiff,
                    intent: OpenIntent::Reveal,
                    role: AgentRole::Unknown,
                })?;
                Err(LayoutError::UnsupportedHost)
            },
            cx,
        );
        assert_eq!(result, Err(LayoutError::UnsupportedHost));
        assert!(shell.areas.layout.menu_tab.is_none());
        shell.notice = None;
        shell
            .update_layout(
                &key,
                |layout| {
                    assert_eq!(layout.tab(tab).unwrap().target(), &Target::Scripts);
                    let side = layout.split(
                        layout.focused_pane(),
                        Position::Right,
                        vessel_app_view::layout::LayoutMode::SplitPanes,
                    )?;
                    layout.drop_tab(
                        tab,
                        side,
                        Position::Center,
                        vessel_app_view::layout::LayoutMode::SplitPanes,
                    )
                },
                cx,
            )
            .unwrap();
        assert!(shell.notice.is_none());
    });
}

#[gpui::test]
fn canonical_replacement_and_keyed_close_notify_the_original_target(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        shell
            .layout_register_retirement(Owner::Scripts, observed)
            .unwrap();
        let key = key();
        let _initial = shell
            .layout_open_target(
                &key,
                Target::Scripts,
                OpenIntent::Reveal,
                Placement::Focused,
                window,
                cx,
            )
            .unwrap();
        let old = shell.prefs.layouts.clone();
        shell.layout_preferences_applied(&old, &[], cx);
        assert_eq!(
            shell.notice.as_deref(),
            Some("CanonicalReplacement:Scripts")
        );
        let tab = shell
            .layout_open_target(
                &key,
                Target::Scripts,
                OpenIntent::Reveal,
                Placement::Focused,
                window,
                cx,
            )
            .unwrap();
        let plan =
            super::super::super::with_layout_at(shell, &key, |layout| layout.request_close(tab))
                .unwrap();
        shell.layout_close_settled(&key, &plan, Ok(()), cx);
        assert_eq!(shell.notice.as_deref(), Some("Closed:Scripts"));
    });
}

#[gpui::test]
fn dirty_refusal_retains_target_and_emits_no_retirement(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        shell
            .layout_register_retirement(Owner::Scripts, observed)
            .unwrap();
        let key = key();
        let tab = shell
            .layout_open_target(
                &key,
                Target::Scripts,
                OpenIntent::Reveal,
                Placement::Focused,
                window,
                cx,
            )
            .unwrap();
        shell
            .update_layout(&key, |layout| layout.set_modified(tab, Modified::Dirty), cx)
            .unwrap();
        let result = shell.update_layout(
            &key,
            |layout| {
                layout.retarget(RetargetRequest {
                    tab,
                    target: Target::WorkingDiff,
                    intent: OpenIntent::Reveal,
                    role: AgentRole::Unknown,
                })
            },
            cx,
        );
        assert_eq!(result, Err(LayoutError::Modified));
        assert!(shell.areas.layout.menu_tab.is_none());
        let target = super::super::super::with_layout_at(shell, &key, |layout| {
            Ok(layout.tab(tab).unwrap().target().clone())
        })
        .unwrap();
        assert_eq!(target, Target::Scripts);
    });
}

#[gpui::test]
fn forgotten_alias_retires_ui_even_when_dirty_layout_survives(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        shell
            .layout_register_retirement(Owner::Scripts, observed)
            .unwrap();
        let key = LayoutKey::Session {
            machine: vessel_app_view::layout::HostAlias::parse("forgotten").unwrap(),
            session: vessel_proto::SessionId::random(),
        };
        let tab = shell
            .layout_open_target(
                &key,
                Target::Scripts,
                OpenIntent::Reveal,
                Placement::Focused,
                window,
                cx,
            )
            .unwrap();
        shell
            .update_layout(&key, |layout| layout.set_modified(tab, Modified::Dirty), cx)
            .unwrap();
        shell.layout_book_applied(
            &vessel_app_view::machine_book::Edit::Forget(
                vessel_app_view::machine_book::Name::parse("forgotten").unwrap(),
            ),
            cx,
        );
        assert_eq!(shell.notice.as_deref(), Some("Unavailable:Scripts"));
        assert!(shell.layout_tab_exists(&key, tab));
        let modified = super::super::super::with_layout_at(shell, &key, |layout| {
            Ok(layout.tab(tab).unwrap().modified())
        })
        .unwrap();
        assert_eq!(modified, Modified::Dirty);
    });
}

#[gpui::test]
fn dismissing_a_compact_sheet_notifies_the_actual_original_file(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _window, cx| {
        shell
            .layout_register_retirement(Owner::Files, observed)
            .unwrap();
        let key = key();
        let target = Target::File {
            root: vessel_proto::workspace::WorkspaceRoot::Project {
                project: vessel_proto::ProjectId::random(),
            },
            path: "original.rs".parse().unwrap(),
        };
        let tab = shell
            .update_layout(
                &key,
                |layout| {
                    layout.open(
                        target.clone(),
                        OpenIntent::Reveal,
                        Placement::Focused,
                        AgentRole::Unknown,
                        vessel_app_view::layout::LayoutMode::Compact,
                    )
                },
                cx,
            )
            .unwrap();
        shell.layout_dismiss_sheet(&key, tab, cx);
        assert_eq!(
            shell.notice.as_deref(),
            Some(format!("Closed:{target:?}").as_str())
        );
        assert!(!shell.layout_tab_exists(&key, tab));
    });
}

#[gpui::test]
fn acknowledged_alias_change_notifies_even_if_local_collision_keeps_both_trees(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        shell
            .layout_register_retirement(Owner::Scripts, observed)
            .unwrap();
        let session = vessel_proto::SessionId::random();
        let old = LayoutKey::Session {
            machine: vessel_app_view::layout::HostAlias::parse("old").unwrap(),
            session,
        };
        let new = LayoutKey::Session {
            machine: vessel_app_view::layout::HostAlias::parse("new").unwrap(),
            session,
        };
        let tab = shell
            .layout_open_target(
                &old,
                Target::Scripts,
                OpenIntent::Reveal,
                Placement::Focused,
                window,
                cx,
            )
            .unwrap();
        let _other = shell
            .layout_open_target(
                &new,
                Target::Scripts,
                OpenIntent::Reveal,
                Placement::Focused,
                window,
                cx,
            )
            .unwrap();
        let before = shell.areas.layout.book.borrow().clone();
        shell.layout_book_applied(
            &vessel_app_view::machine_book::Edit::Rename {
                from: vessel_app_view::machine_book::Name::parse("old").unwrap(),
                to: vessel_app_view::machine_book::Name::parse("new").unwrap(),
            },
            cx,
        );
        assert_eq!(shell.areas.layout.menu_tab.as_ref(), Some(&(old, tab)));
        assert_eq!(*shell.areas.layout.book.borrow(), before);
    });
}

#[gpui::test]
fn failed_first_transaction_creates_no_workspace_or_owner_notice(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _window, cx| {
        shell
            .layout_register_retirement(Owner::Scripts, observed)
            .unwrap();
        let key = key();
        let result: Result<(), LayoutError> =
            shell.update_layout(&key, |_layout| Err(LayoutError::MissingTab), cx);
        assert_eq!(result, Err(LayoutError::MissingTab));
        assert!(
            shell
                .areas
                .layout
                .book
                .borrow()
                .as_ref()
                .unwrap()
                .get(&key)
                .is_none()
        );
        assert!(shell.areas.layout.menu_tab.is_none());
    });
}

fn remember_scripts(shell: &mut Shell) -> (LayoutKey, TabId) {
    let key = key();
    let mut layout = Layout::default();
    let tab = layout
        .open(
            Target::Scripts,
            OpenIntent::Reveal,
            Placement::Focused,
            AgentRole::Unknown,
            vessel_app_view::layout::LayoutMode::SplitPanes,
        )
        .unwrap();
    shell.prefs.layouts = vec![vessel_app_view::layout::RememberedLayout::new(
        key.clone(),
        &layout,
    )];
    assert!(shell.areas.layout.book.borrow().is_none());
    (key, tab)
}

#[gpui::test]
fn cold_remembered_first_retarget_notifies_the_original_owner(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _window, cx| {
        let (key, tab) = remember_scripts(shell);
        shell
            .layout_register_retirement(Owner::Scripts, observed)
            .unwrap();
        retarget(shell, &key, tab, Target::WorkingDiff, cx);
        assert_eq!(shell.notice.as_deref(), Some("Retargeted:Scripts"));
        assert_eq!(shell.areas.layout.menu_tab.as_ref(), Some(&(key, tab)));
    });
}

#[gpui::test]
fn cold_remembered_failed_first_batch_keeps_target_without_a_notice(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _window, cx| {
        let (key, tab) = remember_scripts(shell);
        shell
            .layout_register_retirement(Owner::Scripts, observed)
            .unwrap();
        let result: Result<(), LayoutError> = shell.update_layout(
            &key,
            |layout| {
                let _tab = layout.retarget(RetargetRequest {
                    tab,
                    target: Target::WorkingDiff,
                    intent: OpenIntent::Reveal,
                    role: AgentRole::Unknown,
                })?;
                Err(LayoutError::UnsupportedHost)
            },
            cx,
        );
        assert_eq!(result, Err(LayoutError::UnsupportedHost));
        assert!(shell.areas.layout.menu_tab.is_none());
        let book = shell.areas.layout.book.borrow();
        assert_eq!(
            book.as_ref()
                .unwrap()
                .get(&key)
                .unwrap()
                .tab(tab)
                .unwrap()
                .target(),
            &Target::Scripts
        );
    });
}

#[gpui::test]
fn initializing_a_remembered_cache_alone_does_not_retire_a_panel(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _window, _cx| {
        let (key, tab) = remember_scripts(shell);
        shell
            .layout_register_retirement(Owner::Scripts, observed)
            .unwrap();
        super::super::super::initialized(shell);
        assert!(shell.areas.layout.menu_tab.is_none());
        assert!(shell.layout_tab_exists(&key, tab));
    });
}
