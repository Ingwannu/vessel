//! Canonical delivery must update the retained layout cache before local edits.

use super::*;
use gpui::TestAppContext;
use vessel_app_link::desktop::preferences::Coordinator;
use vessel_app_view::layout::{
    AgentRole, Layout, LayoutKey, LayoutMode, OpenIntent, Placement, RememberedLayout, Target,
};
use vessel_proto::{NodeId, WorkspaceId};

fn key() -> LayoutKey {
    LayoutKey::Workspace {
        node: NodeId::random(),
        workspace: WorkspaceId::random(),
    }
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
    futures_lite::future::block_on(coordinator.flush()).unwrap();
    let snapshot = coordinator.snapshot();
    shell.update_in(cx, |shell, window, cx| {
        apply(shell, &snapshot, window, cx);
        shell
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
    futures_lite::future::block_on(coordinator.flush()).unwrap();
    assert!(
        coordinator
            .current()
            .layouts
            .iter()
            .any(|entry| entry.key == b)
    );
}
