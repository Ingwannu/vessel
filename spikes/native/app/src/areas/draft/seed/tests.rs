//! Intake uses an isolated shell: no link, credentials, filesystem or agent.
use super::*;
use crate::shell::language_tests;
use gpui::TestAppContext;
use vessel_app_view::composer::context::{ChatHistory, ContextId, ContextTitle, Payload};
use vessel_app_view::creation::ProjectSeed;
use vessel_app_view::{Machine, Reach};
use vessel_proto::{NodeId, ProjectId, WorkspaceId};

fn destination(node: NodeId) -> DraftDestination {
    DraftDestination {
        machine: Name::parse("fixture").unwrap(),
        node,
    }
}
fn connect(shell: &mut Shell, node: NodeId) {
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
}
fn history() -> ChatHistory {
    ChatHistory::new(Payload {
        id: ContextId::parse("history").unwrap(),
        title: ContextTitle::parse("Recent chat").unwrap(),
        text: ContextText::parse("retained history").unwrap(),
        url: None,
    })
}
#[gpui::test]
fn occupied_text_refuses_before_navigation_or_context_intake(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        shell.areas.draft.composer.set_text("  ", window, cx);
        let seed = DraftSeed {
            destination: destination(NodeId::random()),
            place: SeedPlace::NewWorkspace(None),
            launch: Launch::Chat,
            text: ContextText::parse("replacement").unwrap(),
            history: Some(history()),
        };
        assert_eq!(
            shell.open_workspace_seed(seed, window, cx),
            Err(SeedProblem::Busy)
        );
        assert_eq!(shell.route, Route::Draft);
        assert_eq!(shell.areas.draft.composer.text(cx), "  ");
        assert!(!shell.composer_has_context(DraftKey::New));
        assert!(shell.areas.draft.creation.workspace.is_none());
    });
}
#[gpui::test]
fn replacement_node_or_existing_workspace_seed_cannot_mutate_top_level_draft(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        connect(shell, NodeId::random());
        assert_eq!(
            shell.open_new_workspace(
                NewWorkspaceSeed::new(destination(NodeId::random()), None),
                window,
                cx
            ),
            Err(SeedProblem::DestinationChanged)
        );
        let seed = DraftSeed {
            destination: destination(shell.fleet.machines[0].node.unwrap()),
            place: SeedPlace::ExistingWorkspace(WorkspaceId::random()),
            launch: Launch::Chat,
            text: ContextText::parse("replacement").unwrap(),
            history: Some(history()),
        };
        assert_eq!(
            shell.open_workspace_seed(seed, window, cx),
            Err(SeedProblem::TargetMismatch)
        );
        assert_eq!(shell.route, Route::Draft);
        assert!(!shell.draft_has_payload(cx));
        assert!(shell.areas.draft.creation.workspace.is_none());
    });
}
#[gpui::test]
fn accepted_seed_preserves_exact_nested_root_and_canonical_history(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let node = NodeId::random();
        connect(shell, node);
        let project = ProjectSeed {
            project: ProjectId::random(),
            root: "/repo/nested/selected".into(),
        };
        let seed = DraftSeed {
            destination: destination(node),
            place: SeedPlace::NewWorkspace(Some(project.clone())),
            launch: Launch::BlankTerminal,
            text: ContextText::parse("initial text").unwrap(),
            history: Some(history()),
        };
        shell.open_workspace_seed(seed, window, cx).unwrap();
        assert_eq!(shell.route, Route::NewWorkspace);
        assert_eq!(shell.areas.draft.folder, "/repo/nested/selected");
        assert_eq!(shell.areas.draft.composer.text(cx), "initial text");
        assert_eq!(
            shell.areas.composer.drafts.text(DraftKey::New),
            "initial text"
        );
        assert!(shell.composer_has_context(DraftKey::New));
        assert_eq!(
            shell.areas.draft.creation.workspace.as_ref().unwrap(),
            &WorkspaceForm {
                destination: destination(node),
                project: Some(project),
                launch: Launch::BlankTerminal
            }
        );
        assert_eq!(
            shell.open_new_workspace(NewWorkspaceSeed::new(destination(node), None), window, cx),
            Err(SeedProblem::Busy)
        );
        assert_eq!(shell.areas.draft.composer.text(cx), "initial text");
    });
}

#[gpui::test]
fn workspace_selection_never_falls_back_and_explicit_host_change_clears_placement(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let original = NodeId::random();
        let other = NodeId::random();
        connect(shell, original);
        let mut second = shell.fleet.machines[0].clone();
        second.name = "second".to_owned();
        second.node = Some(other);
        shell.fleet.machines.push(second);
        shell
            .open_workspace_seed(
                DraftSeed {
                    destination: destination(original),
                    place: SeedPlace::NewWorkspace(Some(ProjectSeed {
                        project: ProjectId::random(),
                        root: "/repo/selected".into(),
                    })),
                    launch: Launch::Chat,
                    text: ContextText::parse("keep input").unwrap(),
                    history: None,
                },
                window,
                cx,
            )
            .unwrap();
        shell.fleet.machines[0].node = Some(NodeId::random());
        shell.settle_draft();
        assert_eq!(shell.areas.draft.machine.as_deref(), Some("fixture"));
        assert!(shell.areas.draft.agent.is_none());
        assert_eq!(
            shell
                .areas
                .draft
                .creation
                .workspace
                .as_ref()
                .unwrap()
                .destination
                .node,
            original
        );
        shell.choose_machine("second".to_owned(), window, cx);
        let form = shell.areas.draft.creation.workspace.as_ref().unwrap();
        assert_eq!(form.destination.node, other);
        assert_eq!(form.destination.machine.as_str(), "second");
        assert!(form.project.is_none());
        assert_eq!(shell.areas.draft.composer.text(cx), "keep input");
        assert_eq!(
            shell.areas.composer.drafts.text(DraftKey::New),
            "keep input"
        );
    });
}
