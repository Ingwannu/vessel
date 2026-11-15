//! Actual frame and owner query agree after closing a maximized empty pane.
use super::*;
use crate::areas::session::Open;
use crate::shell::language_tests;
use gpui::{AppContext as _, Render, TestAppContext};
use vessel_app_kit::ui::{
    Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _, Styled as _, div,
};
use vessel_app_view::layout::{OpenIntent, Placement, Position, Target};
use vessel_proto::SessionId;

struct Probe(Entity<Shell>);

#[gpui::test]
fn retargeted_tab_keeps_its_id_but_refuses_the_original_resource_callback(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let key = LayoutKey::Workspace {
            node: vessel_proto::NodeId::random(),
            workspace: vessel_proto::WorkspaceId::random(),
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
        assert!(shell.layout_tab_matches(&key, tab, &Target::Scripts));
        let _retargeted = shell
            .update_layout(
                &key,
                |layout| {
                    layout.retarget(vessel_app_view::layout::RetargetRequest {
                        tab,
                        target: Target::WorkingDiff,
                        intent: OpenIntent::Reveal,
                        role: vessel_app_view::layout::AgentRole::Unknown,
                    })
                },
                cx,
            )
            .unwrap();
        assert!(shell.layout_tab_exists(&key, tab));
        assert!(!shell.layout_tab_matches(&key, tab, &Target::Scripts));
        assert!(shell.layout_tab_matches(&key, tab, &Target::WorkingDiff));
        let missing = LayoutKey::Workspace {
            node: vessel_proto::NodeId::random(),
            workspace: vessel_proto::WorkspaceId::random(),
        };
        let before = shell.areas.layout.book.borrow().clone();
        assert!(!shell.layout_tab_matches(&missing, tab, &Target::Scripts));
        assert_eq!(*shell.areas.layout.book.borrow(), before);
    });
}
impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.0.update(cx, |shell, cx| {
            let tokens = vessel_app_kit::tokens(cx);
            super::super::draw::frame(
                shell,
                div()
                    .size_full()
                    .debug_selector(|| "layout-survivor".to_owned())
                    .child("retained agent"),
                &tokens,
                window,
                cx,
            )
        })
    }
}
#[gpui::test]
fn closing_a_maximized_empty_pane_restores_surviving_panel_and_poll_visibility(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| {
        Probe(cx.new(|cx| {
            let mut shell = Shell::without_io(window, cx);
            shell.route = crate::routes::Route::Session;
            shell.areas.session.open = Some(Open {
                machine: "isolated".to_owned(),
                session: SessionId::random(),
                title: "retained".to_owned(),
                agent: "fake".to_owned(),
                structured: true,
                live: true,
                archived_at: None,
                binding: None,
            });
            shell
        }))
    });
    cx.simulate_resize(gpui::size(gpui::px(1280.0), gpui::px(900.0)));
    probe.update_in(cx, |probe, window, cx| {
        probe.0.update(cx, |shell, cx| {
            let key = super::super::key(shell).unwrap();
            let session = shell.areas.session.open.as_ref().unwrap().session;
            let tab = shell
                .layout_open_target(
                    &key,
                    Target::Agent(session),
                    OpenIntent::Reveal,
                    Placement::Focused,
                    window,
                    cx,
                )
                .unwrap();
            let pane = shell
                .update_layout(
                    &key,
                    |layout| layout.split(PaneId::MAIN, Position::Right, LayoutMode::SplitPanes),
                    cx,
                )
                .unwrap();
            shell.areas.layout.maximized = Some((key.clone(), pane));
            assert!(!shell.layout_panel_visible(&key, tab, window));
            shell
                .update_layout(&key, |layout| layout.close_empty_pane(pane), cx)
                .unwrap();
            assert!(shell.layout_panel_visible(&key, tab, window));
        });
    });
    cx.run_until_parked();
    assert!(cx.debug_bounds("layout-survivor").is_some());
}
