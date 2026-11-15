//! The first subscription must rebase the actual pre-subscription window cache.
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
