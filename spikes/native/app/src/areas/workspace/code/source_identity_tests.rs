//! Acquisition equality is deliberately independent of capability liveness.
//! Start from genuine framed source receipts; mutate only copied identity facts.
use super::*;
use vessel_app_view::workspace::file_scope::FileScope;

#[gpui::test]
fn same_source_compares_each_original_fact_but_never_liveness(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, _, _| {
        let (session, project) = prepare(shell, &scene.context());
        root(&mut scene, shell, session, project);
        ready(&mut scene, shell);
        let original = shell.shown_changes_context().unwrap();
        let copy = original.clone();
        assert!(original.same_source(&copy));
        let mut root_ticket = copy.clone();
        root_ticket.root.ticket = shell.areas.workspace.code.tickets.issue().unwrap();
        let mut summary_ticket = copy.clone();
        summary_ticket.ticket = shell.areas.workspace.code.tickets.issue().unwrap();
        let mut session = copy.clone();
        session.root.session = SessionId::random();
        let mut comparison = copy.clone();
        comparison.comparison = ScopeKind::Staged;
        let mut root = copy.clone();
        root.scope = FileScope::new(
            scene.context(),
            WorkspaceRoot::Project {
                project: ProjectId::random(),
            },
        );
        let replacement = scene
            .reconnect(
                shell.fixture_link(),
                vessel_app_link::testing::Reconnect::SameNode,
            )
            .unwrap();
        let mut context = copy.clone();
        context.root.context = replacement;
        for other in [
            root_ticket,
            summary_ticket,
            session,
            comparison,
            root,
            context,
        ] {
            assert!(!original.same_source(&other));
            assert!(!other.same_source(&original));
        }
        // Reconnection retires the original real admission. Equal acquisition
        // identity must not fabricate currentness from equal copied fields.
        assert!(original.same_source(&copy));
        assert!(!shell.shown_changes_is_current(&original));
        assert!(!shell.shown_changes_is_current(&copy));
        scene.close(shell.fixture_link()).unwrap();
    });
}
