//! Real isolated editors prove that a scoped draft never aliases global New.
use super::*;
use crate::shell::language_tests;
use gpui::TestAppContext;
use vessel_app_view::composer::context::{ContextProblem, ContextText};
use vessel_app_view::machine_book::Name;
use vessel_app_view::{Machine, Reach};
use vessel_proto::{NodeId, WorkspaceId};

fn connect(shell: &mut Shell, node: NodeId) -> DraftDestination {
    shell.fleet.machines.push(Machine {
        connection_epoch: None,
        name: "fixture".to_owned(),
        reach: Reach::Online,
        sessions: Vec::new(),
        agents: Vec::new(),
        protocol: None,
        version: None,
        node: Some(node),
    });
    DraftDestination {
        machine: Name::parse("fixture").unwrap(),
        node,
    }
}
fn seed(destination: DraftDestination, workspace: WorkspaceId, text: &str) -> DraftSeed {
    DraftSeed {
        destination,
        place: SeedPlace::ExistingWorkspace(workspace),
        launch: Launch::BlankTerminal,
        text: ContextText::parse(text).unwrap(),
        history: None,
    }
}
#[gpui::test]
fn exact_workspace_and_draft_are_required_and_busy_reseed_preserves_the_editor(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let node = NodeId::random();
        let workspace = WorkspaceId::random();
        let draft = DraftId::random();
        let destination = connect(shell, node);
        let key = LayoutKey::Workspace { node, workspace };
        shell
            .areas
            .draft
            .composer
            .set_text("global text", window, cx);
        shell
            .seed_draft(
                draft,
                seed(destination.clone(), workspace, "scoped text"),
                window,
                cx,
            )
            .unwrap();
        assert_eq!(
            shell.workspace_draft_field(&key, draft).unwrap().text(cx),
            "scoped text"
        );
        assert_eq!(shell.areas.draft.composer.text(cx), "global text");
        assert!(
            shell
                .workspace_draft_field(
                    &LayoutKey::Workspace {
                        node: NodeId::random(),
                        workspace
                    },
                    draft
                )
                .is_none()
        );
        assert!(
            shell
                .workspace_draft_field(
                    &LayoutKey::Workspace {
                        node,
                        workspace: WorkspaceId::random()
                    },
                    draft
                )
                .is_none()
        );
        assert!(
            shell
                .workspace_draft_field(&key, DraftId::random())
                .is_none()
        );
        assert!(shell.workspace_draft_context_scope(&key, draft).is_none());
        assert_eq!(
            shell.seed_draft(
                draft,
                seed(destination, workspace, "replacement"),
                window,
                cx
            ),
            Err(SeedProblem::Busy)
        );
        assert_eq!(
            shell.workspace_draft_field(&key, draft).unwrap().text(cx),
            "scoped text"
        );
        assert_eq!(
            shell.route,
            crate::routes::Route::Draft,
            "intake must precede opening the layout"
        );
    });
}
#[gpui::test]
fn bounded_intake_refuses_new_drafts_without_evicting_existing_content(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let node = NodeId::random();
        let workspace = WorkspaceId::random();
        let destination = connect(shell, node);
        let retained = DraftId::random();
        shell
            .seed_draft(
                retained,
                seed(destination.clone(), workspace, "retain"),
                window,
                cx,
            )
            .unwrap();
        for _ in 1..Drafts::MAX_DRAFTS {
            shell
                .seed_draft(
                    DraftId::random(),
                    seed(destination.clone(), workspace, ""),
                    window,
                    cx,
                )
                .unwrap();
        }
        assert_eq!(
            shell.seed_draft(
                DraftId::random(),
                seed(destination, workspace, "overflow"),
                window,
                cx
            ),
            Err(SeedProblem::Context(ContextProblem::Capacity))
        );
        assert_eq!(
            shell
                .workspace_draft_field(&LayoutKey::Workspace { node, workspace }, retained)
                .unwrap()
                .text(cx),
            "retain"
        );
    });
}

#[gpui::test]
fn mode_comes_only_from_the_exact_drafts_retained_launch(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let node = NodeId::random();
        let workspace = WorkspaceId::random();
        let destination = connect(shell, node);
        let key = LayoutKey::Workspace { node, workspace };
        for (launch, expected) in [
            (Launch::Chat, Mode::Chat),
            (Launch::BlankTerminal, Mode::Terminal),
            (
                Launch::Profile(vessel_proto::terminal_profiles::Id::parse("configured").unwrap()),
                Mode::Terminal,
            ),
        ] {
            let draft = DraftId::random();
            let mut input = seed(destination.clone(), workspace, "");
            input.launch = launch;
            shell.seed_draft(draft, input, window, cx).unwrap();
            assert_eq!(shell.workspace_draft_mode(&key, draft), Some(expected));
            assert_eq!(
                shell.workspace_draft_mode(
                    &LayoutKey::Workspace {
                        node: NodeId::random(),
                        workspace
                    },
                    draft
                ),
                None
            );
        }
        assert_eq!(shell.workspace_draft_mode(&key, DraftId::random()), None);
    });
}
