//! Actual framed root/summary/diff replies enter the real CodePanel owner.
use super::*;
use gpui::TestAppContext;
use vessel_app_link::{
    Event,
    testing::Scenario,
    workspace::code_source::{Answer, Request},
};
use vessel_app_view::{connection::ConnectionContext, machine_book::Name};
use vessel_proto::{
    Call, HarnessKind, Outcome, ProjectId, Reply, SessionInfo, SessionMode, SessionSpec,
    workspace::{ChangeStatus, DiffContent, FileChange, FileDiff, SessionWorkspace},
};

fn prepare(shell: &mut Shell, context: &ConnectionContext) -> (SessionId, ProjectId) {
    shell.route = crate::routes::Route::Session;
    let session = SessionId::random();
    shell.areas.session.open = Some(crate::areas::session::Open {
        machine: context.machine().as_str().to_owned(),
        session,
        title: "source fixture".to_owned(),
        agent: "fake".to_owned(),
        structured: true,
        live: true,
    });
    let mut machine =
        vessel_app_view::Machine::unreachable(context.machine().as_str().to_owned(), "fixture");
    machine.node = Some(context.node());
    machine.connection_epoch = Some(context.epoch());
    shell.fleet.machines = vec![machine];
    shell.areas.workspace.code.open = true;
    shell.code_follow();
    (session, ProjectId::random())
}
fn metadata(session: SessionId, project: ProjectId) -> SessionInfo {
    let mut info = SessionInfo::new(
        session,
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        chrono::DateTime::UNIX_EPOCH,
    );
    info.workspace = Some(Box::new(SessionWorkspace {
        project,
        worktree: None,
        branch: None,
    }));
    info
}
fn summary() -> DiffSummary {
    DiffSummary {
        from: None,
        more: false,
        files: vec![FileChange {
            path: "a.rs".parse().unwrap(),
            old_path: None,
            status: ChangeStatus::Modified,
            additions: Some(1),
            deletions: Some(1),
            binary: false,
        }],
    }
}
fn answer(scene: &mut Scenario, shell: &Shell) -> Answer {
    const MAX_EVENTS: usize = 32;
    for _ in 0..MAX_EVENTS {
        if let Event::CodeSource(answer) = scene.event(shell.fixture_link()).unwrap() {
            return *answer;
        }
    }
    panic!("expected bounded code-source event");
}
fn root(scene: &mut Scenario, shell: &mut Shell, session: SessionId, project: ProjectId) {
    assert!(
        matches!(scene.request(shell.fixture_link()).unwrap(), Call::GetSession(ask) if ask.session == session)
    );
    scene
        .reply(
            shell.fixture_link(),
            Outcome::Ok(Reply::GetSession(metadata(session, project))),
        )
        .unwrap();
    shell.code_source_answer(answer(scene, shell));
    assert!(
        matches!(scene.request(shell.fixture_link()).unwrap(), Call::GetChanges(ask) if ask.root == WorkspaceRoot::Project { project })
    );
}
fn ready(scene: &mut Scenario, shell: &mut Shell) {
    scene
        .reply(
            shell.fixture_link(),
            Outcome::Ok(Reply::GetChanges(summary())),
        )
        .unwrap();
    shell.code_source_answer(answer(scene, shell));
    let shown = shell.shown_changes_context().unwrap();
    assert_eq!(shown.connection(), &scene.context());
    assert!(shell.shown_changes_is_current(&shown));
}
fn setup(cx: &mut TestAppContext) -> (gpui::Entity<Shell>, &mut gpui::VisualTestContext, Scenario) {
    crate::shell::language_tests::init(cx);
    let (link, scene) = Scenario::start(&Name::parse("fixture").unwrap()).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    (shell, cx, scene)
}
#[gpui::test]
fn queued_summary_cannot_install_after_original_retirement(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, _, _| {
        let (session, project) = prepare(shell, &scene.context());
        root(&mut scene, shell, session, project);
        scene
            .reply(
                shell.fixture_link(),
                Outcome::Ok(Reply::GetChanges(summary())),
            )
            .unwrap();
        let held = answer(&mut scene, shell);
        assert!(
            held.guard
                .as_ref()
                .unwrap()
                .validate(&scene.context())
                .is_ok()
        );
        scene.retire(shell.fixture_link()).unwrap();
        shell.code_source_answer(held);
        assert!(shell.shown_changes_context().is_none());
        assert!(!matches!(
            shell.areas.workspace.code.changes.summary,
            Summary::Ready(_)
        ));
        scene.close(shell.fixture_link()).unwrap();
    });
}
#[gpui::test]
fn installed_summary_getter_observes_retirement_without_fleet_delivery(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, _, _| {
        let (session, project) = prepare(shell, &scene.context());
        root(&mut scene, shell, session, project);
        ready(&mut scene, shell);
        let shown = shell.shown_changes_context().unwrap();
        scene.retire(shell.fixture_link()).unwrap();
        assert!(shell.shown_changes_context().is_none());
        assert!(!shell.shown_changes_is_current(&shown));
        assert!(matches!(
            shell.areas.workspace.code.changes.summary,
            Summary::Ready(_)
        ));
        scene.close(shell.fixture_link()).unwrap();
    });
}
#[gpui::test]
fn refresh_and_comparison_aba_cannot_reuse_the_old_ready_stamp(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, _, _| {
        let (session, project) = prepare(shell, &scene.context());
        root(&mut scene, shell, session, project);
        ready(&mut scene, shell);
        let old = shell.shown_changes_context().unwrap();
        shell.reread_changes();
        assert!(!shell.shown_changes_is_current(&old));
        assert!(matches!(scene.request(shell.fixture_link()).unwrap(), Call::GetChanges(_)));
        scene.reply(shell.fixture_link(), Outcome::Ok(Reply::GetChanges(summary()))).unwrap();
        let held = answer(&mut scene, shell);
        shell.areas.workspace.code.changes.scope = ScopeKind::Staged;
        shell.source_summary();
        assert!(matches!(scene.request(shell.fixture_link()).unwrap(), Call::GetChanges(ask) if ask.scope == ScopeKind::Staged.scope()));
        scene.reply(shell.fixture_link(), Outcome::Ok(Reply::GetChanges(summary()))).unwrap();
        let staged = answer(&mut scene, shell);
        shell.areas.workspace.code.changes.scope = ScopeKind::Uncommitted;
        shell.source_summary();
        assert!(matches!(scene.request(shell.fixture_link()).unwrap(), Call::GetChanges(_)));
        shell.code_source_answer(held);
        shell.code_source_answer(staged);
        assert!(shell.shown_changes_context().is_none());
        ready(&mut scene, shell);
        assert!(!shell.shown_changes_is_current(&old));
        scene.close(shell.fixture_link()).unwrap();
    });
}
#[gpui::test]
fn diff_and_legacy_answers_cannot_cross_the_parent_summary(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, _, _| {
        let (session, project) = prepare(shell, &scene.context());
        root(&mut scene, shell, session, project);
        ready(&mut scene, shell);
        let path = "a.rs".parse().unwrap();
        shell.read_diff(&path, None);
        assert!(matches!(scene.request(shell.fixture_link()).unwrap(), Call::GetFileDiff(ask) if ask.path == path));
        scene.reply(shell.fixture_link(), Outcome::Ok(Reply::GetFileDiff(FileDiff {
            path: path.clone(), old_path: None, status: ChangeStatus::Modified,
            mode: None, content: DiffContent::Unchanged, more_from: None }))).unwrap();
        let page = answer(&mut scene, shell);
        let request = page.request.clone();
        shell.code_source_answer(page);
        let shown = shell.shown_diff_context(&path).unwrap();
        assert!(shell.shown_diff_is_current(&shown));
        shell.reread_changes();
        assert!(!shell.shown_diff_is_current(&shown));
        let Request::Diff { ask, .. } = request else { panic!("actual diff request"); };
        shell.diff_read(&ask, Ok(FileDiff { path: path.clone(), old_path: None, status: ChangeStatus::Modified,
            mode: None, content: DiffContent::Unchanged, more_from: None }));
        assert!(shell.shown_diff_context(&path).is_none());
        assert!(matches!(scene.request(shell.fixture_link()).unwrap(), Call::GetChanges(_)));
        ready(&mut scene, shell);
        assert!(!shell.shown_diff_is_current(&shown));
        scene.close(shell.fixture_link()).unwrap();
    });
}
