//! The real node and fake ACP harness exercise the same pending/assigned wire
//! and durable workspace paths used by the page.

use std::time::Duration;

use vessel_client::NodeClient;
use vessel_proto::automation::{
    AfterRun, Cadence, CatchUp, RunArchive, RunOutcome, SCHEDULE_ISOLATION_PROTOCOL,
    ScheduleDefinition, ScheduleTarget,
};
use vessel_proto::registry::{
    GetRegistry, RegistrySnapshot, WorkspaceBacking, WorkspaceDescriptor, WorkspaceIsolation,
    WorkspaceLifecycle, WorkspaceResource,
};
use vessel_proto::workspace::{AddProject, SessionPlace, SetupChoice, WorktreeBranch};
use vessel_proto::{ProjectId, ScheduleId, SessionId};

use crate::support::{Fixture, step, structured_spec, within};
use crate::workspace::{lookup, repository};

fn task(
    fixture: &Fixture,
    project: ProjectId,
    isolation: WorkspaceIsolation,
    after: AfterRun,
) -> ScheduleDefinition {
    let mut spec =
        structured_spec(fixture, None).with_place(SessionPlace::Project { project, dir: None });
    spec.cwd = None;
    ScheduleDefinition {
        name: Some("work on a separate workspace".parse().unwrap()),
        cadence: Cadence::Every {
            seconds: 3600.try_into().unwrap(),
            anchor: None,
        },
        target: ScheduleTarget::NewSession {
            spec: Box::new(spec),
            after,
            isolation: Some(isolation),
        },
        prompt: "plain".parse().unwrap(),
        catch_up: CatchUp::Skip,
        max_runs: None,
        ends_at: None,
    }
}

async fn snapshot(client: &NodeClient) -> RegistrySnapshot {
    client
        .call(GetRegistry { changed_from: None })
        .await
        .unwrap()
}

fn workspace_of(registry: &RegistrySnapshot, session: SessionId) -> &WorkspaceDescriptor {
    let resource = WorkspaceResource::Session { session };
    let membership = registry
        .memberships
        .iter()
        .find(|item| item.resource == resource)
        .unwrap();
    registry
        .workspaces
        .iter()
        .find(|item| item.id == membership.workspace)
        .unwrap()
}

async fn run(
    client: &NodeClient,
    schedule: ScheduleId,
    disposition: Option<RunArchive>,
) -> SessionId {
    let asked = client.run_schedule(schedule).await.unwrap();
    let RunOutcome::Started { session } = asked.outcome else {
        panic!("the run must start: {asked:?}");
    };
    step("isolated run's turn and archive result", async {
        loop {
            let current = client.schedule(schedule).await.unwrap().last_run.unwrap();
            if current.number == asked.number
                && matches!(current.outcome, RunOutcome::Finished { .. })
            {
                assert_eq!(current.archive, disposition);
                break;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await;
    session
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scheduled_local_members_keep_distinct_ids_and_managed_archives_remove_only_their_backing()
{
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.harness_lookup = lookup(&fixture))
            .await;
        let owner = fixture.connect(&node).await;
        let folder = tempfile::tempdir().unwrap();
        repository(folder.path());
        let project = owner
            .call(AddProject {
                path: folder.path().to_owned(),
                name: None,
            })
            .await
            .unwrap();
        let definition = task(
            &fixture,
            project.id,
            WorkspaceIsolation::Local,
            AfterRun::Keep,
        );
        let local = owner.create_schedule(definition.clone(), false).await;
        if SCHEDULE_ISOLATION_PROTOCOL.is_none() {
            assert!(matches!(
                local,
                Err(vessel_rpc::CallError::UnassignedProtocol {
                    method: vessel_proto::Method::CreateSchedule
                })
            ));
            assert!(owner.schedules().await.unwrap().is_empty());
            assert!(snapshot(&owner).await.workspaces.is_empty());
            return;
        }
        let local = local.unwrap();

        let mut old_client = definition;
        if let ScheduleTarget::NewSession { isolation, .. } = &mut old_client.target {
            *isolation = None;
        }
        assert!(
            owner
                .update_schedule(local.id, local.revision, old_client)
                .await
                .is_err()
        );

        let first = run(&owner, local.id, None).await;
        let second = run(&owner, local.id, None).await;
        assert_ne!(first, second);
        let registry = snapshot(&owner).await;
        let first_local = workspace_of(&registry, first);
        let second_local = workspace_of(&registry, second);
        assert_ne!(first_local.id, second_local.id);
        assert_eq!(first_local.path, project.root);
        assert_eq!(second_local.path, project.root);
        assert_eq!(first_local.lifecycle, WorkspaceLifecycle::Active);
        assert_eq!(second_local.lifecycle, WorkspaceLifecycle::Active);

        let managed = owner
            .create_schedule(
                task(
                    &fixture,
                    project.id,
                    WorkspaceIsolation::Worktree {
                        branch: WorktreeBranch::New {
                            name: None,
                            base: None,
                        },
                        setup: SetupChoice::Skip,
                    },
                    AfterRun::Archive,
                ),
                false,
            )
            .await
            .unwrap();
        let first = run(&owner, managed.id, Some(RunArchive::Archived)).await;
        let second = run(&owner, managed.id, Some(RunArchive::Archived)).await;
        let registry = snapshot(&owner).await;
        let one = workspace_of(&registry, first);
        let two = workspace_of(&registry, second);
        assert_ne!(one.id, two.id);
        assert_ne!(one.backing, two.backing);
        for workspace in [one, two] {
            assert!(matches!(
                workspace.backing,
                WorkspaceBacking::Managed { .. }
            ));
            assert!(matches!(
                workspace.lifecycle,
                WorkspaceLifecycle::Archived { .. }
            ));
            assert!(!workspace.path.exists());
        }
        assert!(project.root.join("README.md").exists());
    })
    .await;
}
