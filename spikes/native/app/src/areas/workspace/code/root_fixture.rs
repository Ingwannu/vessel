//! Test-only original root acquisition through the one real framed Scenario.
//! Local reducer fixtures cannot turn ConnectionContext facts into a live guard.
use super::{Place, Shell, Summary, Tab, Target, file_open::FileOpenContext};
use gpui::{Entity, TestAppContext, VisualTestContext};
use vessel_app_kit::ui::Context;
use vessel_app_link::{Event, testing::Scenario, workspace::code_source::Answer};
use vessel_app_view::{connection::ConnectionContext, machine_book::Name};
use vessel_proto::{
    Call, HarnessKind, Outcome, ProjectId, Reply, SessionId, SessionInfo, SessionMode, SessionSpec,
    workspace::{DiffSummary, SessionWorkspace, WorkspaceRoot},
};

pub(super) fn setup<'a>(
    cx: &'a mut TestAppContext,
    name: &str,
) -> (Entity<Shell>, &'a mut VisualTestContext, Scenario) {
    crate::shell::language_tests::init(cx);
    let (link, scene) = Scenario::start(&Name::parse(name).unwrap()).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    (shell, cx, scene)
}
pub(super) fn metadata(session: SessionId, project: ProjectId) -> SessionInfo {
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
fn publish(shell: &mut Shell, context: &ConnectionContext) {
    let mut machine =
        vessel_app_view::Machine::unreachable(context.machine().as_str().to_owned(), "fixture");
    machine.node = Some(context.node());
    machine.connection_epoch = Some(context.epoch());
    shell.fleet.machines = vec![machine];
}
pub(super) fn acquire(
    shell: &mut Shell,
    scene: &mut Scenario,
    root: WorkspaceRoot,
    cx: &mut Context<'_, Shell>,
) -> FileOpenContext {
    let session = SessionId::random();
    let context = scene.context();
    shell.route = crate::routes::Route::Session;
    shell.areas.session.open = Some(crate::areas::session::Open {
        machine: context.machine().as_str().to_owned(),
        session,
        title: "fixture".into(),
        agent: "fake".into(),
        structured: true,
        live: true,
    });
    shell.areas.workspace.code.target = Some(Target {
        admission: None,
        machine: context.machine().as_str().to_owned(),
        session,
        place: Place::Reading,
        cwd: None,
    });
    refresh(shell, scene, root, cx)
}
pub(super) fn refresh(
    shell: &mut Shell,
    scene: &mut Scenario,
    root: WorkspaceRoot,
    cx: &mut Context<'_, Shell>,
) -> FileOpenContext {
    let context = scene.context();
    publish(shell, &context);
    let session = shell.areas.workspace.code.target.as_ref().unwrap().session;
    let prior_tab = shell.areas.workspace.code.tab;
    // Acquire only metadata/comparison here; each caller still owns its file
    // reducer's reads. No synthetic validity branch or extra transport exists.
    shell.areas.workspace.code.tab = Tab::Changes;
    assert!(shell.source_root(context.machine().as_str(), session));
    assert!(
        matches!(scene.request(shell.fixture_link()).unwrap(), Call::GetSession(ask) if ask.session == session)
    );
    let mut info = match root {
        WorkspaceRoot::Project { project } => metadata(session, project),
        WorkspaceRoot::Worktree { worktree } => {
            let mut info = metadata(session, ProjectId::random());
            info.workspace.as_mut().unwrap().worktree = Some(worktree);
            info
        }
    };
    info.spec.cwd = Some("/fixture".into());
    scene
        .reply(shell.fixture_link(), Outcome::Ok(Reply::GetSession(info)))
        .unwrap();
    let received = answer(shell, scene, cx);
    shell.code_source_answer(received);
    assert!(
        matches!(scene.request(shell.fixture_link()).unwrap(), Call::GitChanges(ask) if ask.root == root)
    );
    scene
        .reply(
            shell.fixture_link(),
            Outcome::Ok(Reply::GitChanges(DiffSummary {
                from: None,
                more: false,
                files: Vec::new(),
            })),
        )
        .unwrap();
    let received = answer(shell, scene, cx);
    shell.code_source_answer(received);
    assert!(matches!(
        shell.areas.workspace.code.changes.summary,
        Summary::Ready(_)
    ));
    shell.areas.workspace.code.tab = prior_tab;
    let opening = shell.file_open_context().unwrap();
    assert_eq!(opening.scope().connection(), &context);
    assert_eq!(opening.scope().root(), root);
    opening
}
fn answer(shell: &mut Shell, scene: &mut Scenario, cx: &mut Context<'_, Shell>) -> Answer {
    for _ in 0..32 {
        match scene.event(shell.fixture_link()).unwrap() {
            Event::CodeSource(answer) => return *answer,
            Event::FileOperation(reply) => shell.file_operation_answered(reply, cx),
            _ => {}
        }
    }
    panic!("bounded original root answer");
}
