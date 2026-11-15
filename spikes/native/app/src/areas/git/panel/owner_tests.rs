//! Actual Git registration and layout transactions, not marker-only callbacks.
use super::*;
use crate::shell::{Shell, language_tests};
use gpui::TestAppContext;
use vessel_app_kit::ui::Context;
use vessel_app_link::workspace::GitWrite;
use vessel_app_link::{Command, Link};
use vessel_app_view::layout::{AgentRole, LayoutError, OpenIntent, RetargetRequest, Target};
use vessel_app_view::workspace::git::{RequestKey, WriteIntent};
use vessel_app_view::workspace::git_quick::GitAction;
use vessel_proto::workspace::{Push, WorkspaceRoot};

struct StoredWrite {
    key: RequestKey,
    payload: GitWrite,
    intent: WriteIntent,
}
fn pending(shell: &mut Shell, root: WorkspaceRoot) -> StoredWrite {
    let stored = StoredWrite {
        key: shell.areas.git.keys.issue().unwrap(),
        payload: GitWrite::Push(Push {
            root,
            remote: None,
            set_upstream: false,
        }),
        intent: WriteIntent::Quick(GitAction::Push),
    };
    assert!(
        shell
            .areas
            .git
            .writing
            .begin_action(stored.key, stored.intent.clone())
    );
    shell.areas.git.pending = Some((stored.key, stored.payload.clone()));
    stored
}
fn retained(shell: &Shell, stored: &StoredWrite) {
    assert_eq!(
        shell.areas.git.pending,
        Some((stored.key, stored.payload.clone()))
    );
    assert_eq!(shell.areas.git.writing.intent(), Some(&stored.intent));
    assert!(shell.areas.git.writing.busy());
}

fn retarget(
    shell: &mut Shell,
    address: &PanelAddress,
    target: Target,
    cx: &mut Context<'_, Shell>,
) {
    let changed = shell
        .update_layout(
            &address.workspace,
            |layout| {
                layout.retarget(RetargetRequest {
                    tab: address.tab,
                    target,
                    intent: OpenIntent::Reveal,
                    role: AgentRole::Unknown,
                })
            },
            cx,
        )
        .unwrap();
    assert_eq!(changed, address.tab);
}
fn empty(inbox: &mut tokio::sync::mpsc::Receiver<Command>) {
    assert!(matches!(
        inbox.try_recv(),
        Err(tokio::sync::mpsc::error::TryRecvError::Empty)
    ));
}

#[gpui::test]
fn missing_startup_refuses_a_valid_visible_pane_before_any_new_read(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (link, mut inbox) = Link::detached(8).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        let (old, _) = super::target_tests::prepare_uninitialized(shell, window, cx);
        while inbox.try_recv().is_ok() {}
        let prior = crate::areas::git::Target {
            machine: "fixture".to_owned(),
            root: old.repository.root,
        };
        shell.areas.git.target = Some(prior.clone());
        let stored = pending(shell, old.repository.root);
        assert!(shell.layout_panel_visible(&old.address.workspace, old.address.tab, window));
        assert_eq!(
            shell.areas.git.panels.owner_problem(),
            Some(ReadFailure::OwnerUnavailable { cause: None })
        );
        shell.git_panel_refresh(old.address.clone(), old.target.clone(), window, cx);
        shell.git_panel_compare(&old, ComparisonMode::Committed, window, cx);
        shell.git_follow(
            crate::areas::git::Target {
                machine: "fixture".to_owned(),
                root: WorkspaceRoot::Project {
                    project: vessel_proto::ProjectId::random(),
                },
            },
            cx,
        );
        shell.git_read_status();
        shell.git_read_branches();
        shell.git_read_stashes();
        assert_eq!(shell.areas.git.target.as_ref(), Some(&prior));
        retained(shell, &stored);
        assert_eq!(
            shell.areas.git.panels.entries[&old.address].instance,
            Some(old.instance)
        );
        assert!(shell.git_command_entries().is_none());
        empty(&mut inbox);
    });
}

#[gpui::test]
fn duplicate_registration_preserves_the_refusal_and_does_not_admit_fresh_work(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (link, mut inbox) = Link::detached(8).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        let (old, _) = super::target_tests::prepare(shell, window, cx);
        while inbox.try_recv().is_ok() {}
        assert_eq!(
            crate::areas::git::start(shell, cx),
            Err(LayoutError::InvalidState)
        );
        assert_eq!(
            shell.areas.git.panels.owner_problem(),
            Some(ReadFailure::OwnerUnavailable {
                cause: Some(LayoutError::InvalidState),
            })
        );
        shell.git_panel_open(old.address.clone(), old.target.clone(), window, cx);
        assert!(!shell.areas.git.panels.entries.contains_key(&old.address));
        assert!(shell.git_command_entries().is_none());
        empty(&mut inbox);
    });
}

#[gpui::test]
fn real_retarget_away_and_back_requires_a_fresh_git_owner_instance(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (link, mut inbox) = Link::detached(8).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, window, cx| {
        let (old, ticket) = super::target_tests::prepare(shell, window, cx);
        while inbox.try_recv().is_ok() {}
        retarget(shell, &old.address, Target::Scripts, cx);
        assert!(!shell.areas.git.panels.entries.contains_key(&old.address));
        retarget(shell, &old.address, Target::WorkingDiff, cx);
        assert!(shell.layout_tab_matches(
            &old.address.workspace,
            old.address.tab,
            &Target::WorkingDiff
        ));
        shell.git_panel_open(old.address.clone(), old.target.clone(), window, cx);
        let fresh = shell.areas.git.panels.entries[&old.address].instance;
        assert_ne!(fresh, Some(old.instance));
        assert!(fresh.is_some());
        assert!(matches!(
            inbox.try_recv().unwrap(),
            Command::GitPanel(vessel_app_link::git_panel::Ask::Locate(_))
        ));
        shell.git_panel_compare(&old, ComparisonMode::Committed, window, cx);
        empty(&mut inbox);
        shell.git_panel_answer(
            vessel_app_link::git_panel::Answer {
                address: old.address.clone(),
                context: old.context,
                outcome: vessel_app_link::git_panel::Outcome::Status(
                    ticket,
                    Err(ReadFailure::TimedOut),
                ),
            },
            cx,
        );
        let entry = &shell.areas.git.panels.entries[&old.address];
        assert_eq!(entry.instance, fresh);
        assert!(entry.problem.is_none());
        assert!(entry.reads.is_none());
        assert!(matches!(&entry.binding, PanelBinding::Locating(_)));
    });
}

#[gpui::test]
fn failed_transaction_keeps_the_live_git_entry_and_committed_retirement_keeps_pending_write_payload(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let (old, _) = super::target_tests::prepare(shell, window, cx);
        let failed: Result<(), LayoutError> = shell.update_layout(
            &old.address.workspace,
            |layout| {
                let _tab = layout.retarget(RetargetRequest {
                    tab: old.address.tab,
                    target: Target::Scripts,
                    intent: OpenIntent::Reveal,
                    role: AgentRole::Unknown,
                })?;
                Err(LayoutError::UnsupportedHost)
            },
            cx,
        );
        assert_eq!(failed, Err(LayoutError::UnsupportedHost));
        assert_eq!(
            shell.areas.git.panels.entries[&old.address].instance,
            Some(old.instance)
        );
        assert!(shell.layout_tab_matches(
            &old.address.workspace,
            old.address.tab,
            &Target::WorkingDiff
        ));
        let stored = pending(shell, old.repository.root);
        retarget(shell, &old.address, Target::Scripts, cx);
        assert!(!shell.areas.git.panels.entries.contains_key(&old.address));
        retained(shell, &stored);
    });
}

#[gpui::test]
fn original_workspace_is_retired_without_touching_another_workspace_with_the_same_tab_id(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let (old, _) = super::target_tests::prepare(shell, window, cx);
        let mut other = old.address.clone();
        other.workspace = vessel_app_view::layout::LayoutKey::Workspace {
            node: old.context.node(),
            workspace: vessel_proto::WorkspaceId::random(),
        };
        let _previous = shell.areas.git.panels.entries.insert(
            other.clone(),
            Entry::new(old.target.clone(), Some(old.context)),
        );
        retarget(shell, &old.address, Target::Scripts, cx);
        assert!(!shell.areas.git.panels.entries.contains_key(&old.address));
        assert!(shell.areas.git.panels.entries.contains_key(&other));
    });
}

#[gpui::test]
fn cancelled_close_keeps_the_git_view_and_confirmed_close_retires_it(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let (old, _) = super::target_tests::prepare(shell, window, cx);
        let plan = shell
            .update_layout(
                &old.address.workspace,
                |layout| layout.request_close(old.address.tab),
                cx,
            )
            .unwrap();
        shell.layout_close_settled(
            &old.address.workspace,
            &plan,
            Err(LayoutError::UnsupportedHost),
            cx,
        );
        assert!(shell.areas.git.panels.entries.contains_key(&old.address));
        shell.layout_close_settled(&old.address.workspace, &plan, Ok(()), cx);
        assert!(!shell.areas.git.panels.entries.contains_key(&old.address));
        assert!(!shell.layout_tab_exists(&old.address.workspace, old.address.tab));
    });
}
