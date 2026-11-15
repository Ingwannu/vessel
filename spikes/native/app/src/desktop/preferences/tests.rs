//! Canonical delivery must update the retained layout cache before local edits.

use super::*;
use gpui::TestAppContext;
use vessel_app_link::desktop::preferences::Coordinator;
use vessel_app_view::layout::{
    AgentRole, Layout, LayoutKey, LayoutMode, OpenIntent, Placement, RememberedLayout, Target,
};
use vessel_proto::{NodeId, WorkspaceId};

mod multiwindow;

fn key() -> LayoutKey {
    LayoutKey::Workspace {
        node: NodeId::random(),
        workspace: WorkspaceId::random(),
    }
}

struct Temp(std::path::PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _cleanup = std::fs::remove_dir_all(&self.0);
    }
}

#[gpui::test]
fn exact_failed_receipt_reports_once_after_a_newer_success_without_rolling_it_back(
    cx: &mut TestAppContext,
) {
    use vessel_app_view::prefs::{Grouping, Kept, Theme};
    crate::shell::language_tests::init(cx);
    let root = Temp(std::env::temp_dir().join(format!(
        "vessel-pref-app-receipt-{}",
        vessel_proto::SessionId::random()
    )));
    let path = root.0.join("prefs.json");
    std::fs::create_dir_all(&path).unwrap();
    let initial = Prefs {
        kept: Kept::File {
            path: path.display().to_string(),
        },
        ..Prefs::default()
    };
    let coordinator = Coordinator::start(initial.clone()).unwrap();
    let (first, _first_updates) = coordinator.client();
    let (second, _second_updates) = coordinator.client();
    let mut refused = initial.clone();
    refused.settings.appearance.theme = Theme::Light;
    let receipt = first.publish_with_receipt(&refused).unwrap();
    super::super::testing::wait(coordinator.flush()).unwrap();
    std::fs::remove_dir(&path).unwrap();
    let mut accepted = initial;
    accepted.settings.sidebar.group = Grouping::Status;
    second.publish(&accepted).unwrap();
    super::super::testing::wait(coordinator.flush()).unwrap();
    let newer = coordinator.snapshot();
    let older = super::super::testing::wait(receipt.completed()).unwrap();
    assert!(older.revision() < newer.revision());
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let mut failed_again = accepted;
    failed_again.settings.appearance.theme = Theme::Light;
    let later_failure = second.publish_with_receipt(&failed_again).unwrap();
    super::super::testing::wait(coordinator.flush()).unwrap();
    let later_failure = super::super::testing::wait(later_failure.completed()).unwrap();
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        shell.areas.desktop.preferences = Some(first);
        apply(shell, &newer, window, cx);
        assert!(shell.notice.is_none());
        acknowledge(shell, &later_failure, window, cx);
        assert!(shell.notice.is_some());
        shell.notice = None;
        // A different newer failure cannot stand in for this exact receipt.
        acknowledge(shell, &older, window, cx);
        assert_eq!(shell.prefs, *newer.preferences());
        assert_eq!(
            shell.notice.as_deref(),
            Some(shell.words.desktop_shell().preferences_failed())
        );
        shell.notice = None;
        acknowledge(shell, &older, window, cx);
        assert!(
            shell.notice.is_none(),
            "the same failure is not shown again"
        );
        assert_eq!(shell.prefs, *newer.preferences());
    });
}

#[gpui::test]
fn canonical_apply_preserves_another_windows_layout_in_the_next_local_save(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (a, b) = (key(), key());
    let mut layout = Layout::default();
    let tab = layout
        .open(
            Target::NewTab,
            OpenIntent::New,
            Placement::Focused,
            AgentRole::Unknown,
            LayoutMode::SplitPanes,
        )
        .unwrap();
    let initial = Prefs {
        layouts: vec![RememberedLayout::new(a.clone(), &layout)],
        ..Prefs::default()
    };
    let coordinator = Coordinator::start(initial.clone()).unwrap();
    let (local, _local_updates) = coordinator.client();
    let (other, _other_updates) = coordinator.client();
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, _cx| {
        shell.prefs = initial.clone();
        shell.areas.desktop.preferences = Some(local);
        assert!(shell.layout_tab_exists(&a, tab));
    });
    let mut remote = initial;
    remote
        .layouts
        .push(RememberedLayout::new(b.clone(), &Layout::default()));
    other.publish(&remote).unwrap();
    super::super::testing::wait(coordinator.flush()).unwrap();
    let snapshot = coordinator.snapshot();
    shell.update_in(cx, |shell, window, cx| {
        apply(shell, &snapshot, window, cx);
        let _opened = shell
            .update_layout(
                &a,
                |layout| {
                    layout.open(
                        Target::NewTab,
                        OpenIntent::New,
                        Placement::Focused,
                        AgentRole::Unknown,
                        LayoutMode::SplitPanes,
                    )
                },
                cx,
            )
            .unwrap();
        assert!(shell.prefs.layouts.iter().any(|entry| entry.key == b));
    });
    super::super::testing::wait(coordinator.flush()).unwrap();
    assert!(
        coordinator
            .current()
            .layouts
            .iter()
            .any(|entry| entry.key == b)
    );
}

#[gpui::test]
fn subscription_installs_an_already_committed_snapshot_and_rebases_the_actual_old_cache(
    cx: &mut TestAppContext,
) {
    use vessel_app_view::prefs::{PreferenceChange, Theme};
    crate::shell::language_tests::init(cx);
    cx.executor().allow_parking();
    let (a, b) = (key(), key());
    let mut layout = Layout::default();
    let tab = layout
        .open(
            Target::NewTab,
            OpenIntent::New,
            Placement::Focused,
            AgentRole::Unknown,
            LayoutMode::SplitPanes,
        )
        .unwrap();
    let initial = Prefs {
        layouts: vec![RememberedLayout::new(a.clone(), &layout)],
        ..Prefs::default()
    };
    let coordinator = Coordinator::start(initial.clone()).unwrap();
    let (remote, _remote_updates) = coordinator.client();
    let mut committed = initial.clone();
    committed.layouts = vec![RememberedLayout::new(b.clone(), &layout)];
    committed.settings.appearance.theme = Theme::Light;
    remote.publish(&committed).unwrap();
    super::super::testing::wait(coordinator.flush()).unwrap();
    cx.update(|cx| cx.set_global(super::super::testing::desktop(coordinator)));
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        shell.prefs = initial;
        assert!(shell.layout_tab_exists(&a, tab));
        start(shell, window, cx);
        // No later watch publication or foreground drain is needed to install
        // the initial subscription value. A cached removed key must also go.
        assert_eq!(shell.prefs, committed);
        assert!(!shell.layout_tab_exists(&a, tab));
        assert!(shell.layout_tab_exists(&b, tab));
        shell.prefer(&PreferenceChange::Theme(Theme::Dark), window, cx);
    });
    let flushing = cx.update(|_, cx| cx.global::<Desktop>().preferences.flush());
    super::super::testing::wait(flushing).unwrap();
    cx.update(|_, cx| {
        let stored = cx.global::<Desktop>().preferences.current();
        assert_eq!(stored.settings.appearance.theme, Theme::Dark);
        assert_eq!(stored.layouts, committed.layouts);
    });
}
