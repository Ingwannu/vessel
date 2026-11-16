//! Genuine cross-App cache observations use the announced real Scenario.
//! Every response originates in the actual selected-call producer and real frame.
use super::*;
use crate::shell::Shell;
use gpui::TestAppContext;
use std::time::Duration;
use vessel_app_link::{Event, testing::Scenario};
use vessel_app_view::{connection::ConnectionContext, machine_book::Name};
use vessel_proto::{
    Call, Outcome, ProjectId, Reply,
    registry::{
        RegistryRevision, RegistrySnapshot, WorkspaceAutomation, WorkspaceBacking,
        WorkspaceDescriptor, WorkspaceLifecycle, WorkspaceMembership, WorkspacePin,
        WorkspaceResource, WorkspaceVisibility,
    },
    workspace::{GitSupport, ProjectInfo, ProjectList, ProjectState, WorktreeList},
};

const SETTLEMENT_DEADLINE: Duration = Duration::from_secs(5);
const MAX_FIXTURE_EVENTS: usize = 32;

fn initialize(cx: &mut TestAppContext) {
    cx.update(|cx| {
        vessel_app_kit::init(
            cx,
            vessel_app_kit::SchemeChoice::Fixed(vessel_app_kit::Scheme::Dark),
        )
        .unwrap();
    });
}

fn prepare(shell: &mut Shell, context: &ConnectionContext) {
    let mut machine = vessel_app_view::Machine::unreachable(
        context.machine().as_str().to_owned(),
        "fixture session read unavailable",
    );
    machine.node = Some(context.node());
    machine.connection_epoch = Some(context.epoch());
    shell.fleet.machines = vec![machine];
    shell.registry_fleet();
}

fn metadata(context: &ConnectionContext, session: SessionId) -> RegistryData {
    let project = ProjectInfo {
        id: ProjectId::random(),
        name: "registered".parse().unwrap(),
        root: "/fixture/project".into(),
        git: false,
        default_branch: None,
        state: ProjectState::Ready,
        created_at: chrono::DateTime::UNIX_EPOCH,
        revision: 0,
    };
    let workspace = WorkspaceDescriptor {
        id: WorkspaceId::random(),
        project: project.id,
        path: project.root.clone(),
        title: Some("Canonical title".parse().unwrap()),
        automation: WorkspaceAutomation::Allowed,
        pin: WorkspacePin::Pinned,
        backing: WorkspaceBacking::Local,
        lifecycle: WorkspaceLifecycle::Active,
        visibility: WorkspaceVisibility::Foreground,
        created_at: chrono::DateTime::UNIX_EPOCH,
        revision: RegistryRevision::ZERO,
    };
    let member = WorkspaceMembership {
        workspace: workspace.id,
        resource: WorkspaceResource::Session { session },
    };
    RegistryData {
        registry: RegistrySnapshot {
            node: context.node(),
            revision: RegistryRevision::ZERO,
            workspaces: vec![workspace],
            memberships: vec![member],
        },
        projects: ProjectList {
            git: GitSupport::Missing,
            projects: vec![project],
            more: false,
        },
        worktrees: WorktreeList {
            worktrees: Vec::new(),
            more: false,
        },
    }
}

fn serve(scene: &mut Scenario, shell: &Shell, data: RegistryData) {
    assert!(matches!(
        scene.request(shell.fixture_link()).unwrap(),
        Call::GetRegistry(_)
    ));
    scene
        .reply(
            shell.fixture_link(),
            Outcome::Ok(Reply::GetRegistry(data.registry)),
        )
        .unwrap();
    assert!(matches!(
        scene.request(shell.fixture_link()).unwrap(),
        Call::ListProjects(_)
    ));
    scene
        .reply(
            shell.fixture_link(),
            Outcome::Ok(Reply::ListProjects(data.projects)),
        )
        .unwrap();
    assert!(matches!(
        scene.request(shell.fixture_link()).unwrap(),
        Call::ListWorktrees(_)
    ));
    scene
        .reply(
            shell.fixture_link(),
            Outcome::Ok(Reply::ListWorktrees(data.worktrees)),
        )
        .unwrap();
}

fn answer(
    scene: &mut Scenario,
    shell: &Shell,
) -> vessel_app_link::workspace::registry_reads::Response {
    for _ in 0..MAX_FIXTURE_EVENTS {
        if let Event::RegistryRead(response) = scene.event(shell.fixture_link()).unwrap() {
            return response;
        }
    }
    panic!("registry response absent from bounded fixture events");
}

// A successful handler schedules the next long poll. Retire its actual selected
// read, consume the real correlated refusal and await that task's OWN receipt.
// Scenario.close drains Devices only and is not substituted for this proof.
fn drain_poll(scene: &mut Scenario, shell: &mut Shell) {
    assert!(matches!(scene.request(shell.fixture_link()).unwrap(),
        Call::GetRegistry(query) if query.changed_from.is_some()));
    scene.retire(shell.fixture_link()).unwrap();
    let mut response = answer(scene, shell);
    let settlement = response
        .take_settlement()
        .expect("actual spawned registry read");
    shell.registry_read_answer(response);
    settlement
        .wait(SETTLEMENT_DEADLINE)
        .expect("registry cancelled-read normal settlement");
    assert!(shell.areas.workspace.registry.active.is_empty());
    scene.close(shell.fixture_link()).unwrap();
}

#[gpui::test]
fn actual_registry_cache_rechecks_retirement_on_every_query_before_fleet_install(
    cx: &mut TestAppContext,
) {
    initialize(cx);
    let (link, mut scene) = Scenario::start(&Name::parse("registry-fixture").unwrap()).unwrap();
    let context = scene.context();
    let session = SessionId::random();
    let data = metadata(&context, session);
    let workspace = data.registry.workspaces[0].id;
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, _window, _cx| {
        assert_eq!(
            shell
                .areas
                .workspace
                .registry
                .session_workspace(context.node(), session),
            Err(LookupProblem::NotHydrated)
        );
        prepare(shell, &context);
        serve(&mut scene, shell, data);
        let mut response = answer(&mut scene, shell);
        let settled = response.take_settlement().unwrap();
        shell.registry_read_answer(response);
        settled
            .wait(SETTLEMENT_DEADLINE)
            .expect("registry successful read normal settlement");
        let cache = &shell.areas.workspace.registry;
        assert_eq!(cache.phase(&context), Some(RegistryPhase::Ready));
        assert_eq!(
            cache.session_workspace(context.node(), session),
            Ok(Some(workspace))
        );
        assert_eq!(
            cache.session_workspace(context.node(), SessionId::random()),
            Ok(None)
        );
        let rows: Vec<_> = cache.rows(&context).unwrap().collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].workspace().pin, WorkspacePin::Pinned);
        assert_eq!(rows[0].placement().node(), context.node());
        assert_eq!(
            rows[0].workspace().title.as_ref().unwrap().as_str(),
            "Canonical title"
        );
        assert!(cache.workspace_location(context.node(), workspace).is_ok());
        assert!(matches!(
            scene.request(shell.fixture_link()).unwrap(),
            Call::GetRegistry(_)
        ));
        scene.retire(shell.fixture_link()).unwrap();
        // Deliberately leave Fleet on its original context. Actor retirement
        // must invalidate cached metadata before that queued event is applied.
        assert_eq!(
            shell.fleet.machines[0].connection_context(),
            Some(context.clone())
        );
        let cache = &shell.areas.workspace.registry;
        assert_eq!(cache.phase(&context), Some(RegistryPhase::Offline));
        assert!(cache.rows(&context).is_err());
        assert_eq!(
            cache.session_workspace(context.node(), session),
            Err(LookupProblem::Unavailable)
        );
        assert_eq!(
            cache.workspace_location(context.node(), workspace),
            Err(LookupProblem::Unavailable)
        );
        let mut response = answer(&mut scene, shell);
        let settled = response.take_settlement().unwrap();
        shell.registry_read_answer(response);
        settled
            .wait(SETTLEMENT_DEADLINE)
            .expect("registry retired poll normal settlement");
        assert!(shell.areas.workspace.registry.active.is_empty());
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn a_genuine_queued_registry_snapshot_cannot_install_after_original_admission_retires(
    cx: &mut TestAppContext,
) {
    initialize(cx);
    let (link, mut scene) = Scenario::start(&Name::parse("registry-fixture").unwrap()).unwrap();
    let context = scene.context();
    let session = SessionId::random();
    let data = metadata(&context, session);
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, _window, _cx| {
        prepare(shell, &context);
        serve(&mut scene, shell, data);
        let mut held = answer(&mut scene, shell);
        let settled = held.take_settlement().unwrap();
        scene.retire(shell.fixture_link()).unwrap();
        assert_eq!(
            shell.fleet.machines[0].connection_context(),
            Some(context.clone())
        );
        shell.registry_read_answer(held);
        assert_eq!(
            shell.areas.workspace.registry.phase(&context),
            Some(RegistryPhase::Offline)
        );
        assert_eq!(
            shell
                .areas
                .workspace
                .registry
                .session_workspace(context.node(), session),
            Err(LookupProblem::Unavailable)
        );
        assert!(shell.areas.workspace.registry.active.is_empty());
        settled
            .wait(SETTLEMENT_DEADLINE)
            .expect("registry held read normal settlement");
        scene.close(shell.fixture_link()).unwrap();
    });
}

#[gpui::test]
fn same_node_reconnect_requires_the_new_actual_registry_observation(cx: &mut TestAppContext) {
    initialize(cx);
    let (link, mut scene) = Scenario::start(&Name::parse("registry-fixture").unwrap()).unwrap();
    let original = scene.context();
    let old_session = SessionId::random();
    let old_data = metadata(&original, old_session);
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, _window, _cx| {
        prepare(shell, &original);
        serve(&mut scene, shell, old_data);
        let mut held = answer(&mut scene, shell);
        let old_settled = held.take_settlement().unwrap();
        let replacement = scene
            .reconnect(
                shell.fixture_link(),
                vessel_app_link::testing::Reconnect::SameNode,
            )
            .unwrap();
        assert_eq!(replacement.node(), original.node());
        assert_ne!(replacement, original);
        prepare(shell, &replacement);
        shell.registry_read_answer(held);
        assert_eq!(
            shell.areas.workspace.registry.phase(&replacement),
            Some(RegistryPhase::Loading)
        );
        assert_eq!(
            shell
                .areas
                .workspace
                .registry
                .session_workspace(replacement.node(), old_session),
            Err(LookupProblem::NotHydrated)
        );
        let fresh_session = SessionId::random();
        let fresh_data = metadata(&replacement, fresh_session);
        let fresh_workspace = fresh_data.registry.workspaces[0].id;
        serve(&mut scene, shell, fresh_data);
        let mut fresh = answer(&mut scene, shell);
        let new_settled = fresh.take_settlement().unwrap();
        shell.registry_read_answer(fresh);
        old_settled
            .wait(SETTLEMENT_DEADLINE)
            .expect("original read normal settlement");
        new_settled
            .wait(SETTLEMENT_DEADLINE)
            .expect("replacement read normal settlement");
        assert_eq!(
            shell
                .areas
                .workspace
                .registry
                .session_workspace(replacement.node(), fresh_session),
            Ok(Some(fresh_workspace))
        );
        assert_eq!(
            shell
                .areas
                .workspace
                .registry
                .session_workspace(replacement.node(), old_session),
            Ok(None)
        );
        assert!(shell.areas.workspace.registry.rows(&original).is_err());
        drain_poll(&mut scene, shell);
    });
}
