//! Direct guarded-handler tests while the public capability remains pending.
//! The isolated node has no harness lookup, user configuration or real agents.
//! These tests do not advertise a protocol number or claim wire availability.

use super::*;
use crate::{Config, Node, guard::Guarded};
use chrono::DateTime;
use std::{num::NonZeroUsize, path::PathBuf, sync::Arc};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use vessel_client::NodeClient;
use vessel_harness::{HarnessEnv, Lookup};
use vessel_proto::{
    Call, ClientInfo, DeviceId, ErrorCode, NodeId, Origin, OriginBy, Outcome, ProjectId, Reply,
    Request as _, Scope, SessionId, WorkspaceId,
    registry::{
        RegistryRevision, WorkspaceAutomation, WorkspaceBacking, WorkspaceDescriptor,
        WorkspaceLifecycle, WorkspacePin, WorkspaceVisibility,
    },
    workspace_labels::{Assignment, Label, LabelColor},
};
use vessel_rpc::{ClientConfig, Server, memory};
use vessel_store::{DataDir, RegistryRecord};

struct Fixture {
    node: Node,
    guarded: Arc<Guarded>,
    connection: Connection,
    client: NodeClient,
    serving: tokio::task::JoinHandle<()>,
    _temporary: tempfile::TempDir,
}

impl Fixture {
    async fn open() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let mut config = Config::new(
            DataDir::new(temporary.path().join("data")),
            HarnessEnv::new(temporary.path().join("user")),
        );
        config.bind = None;
        config.harness_lookup = Lookup::new(Vec::<PathBuf>::new());
        let node = Node::start(config).await.unwrap();
        let gateway = &node.secure;
        let guarded = Arc::clone(&gateway.service);
        let server = Server::new(
            Arc::clone(&guarded),
            gateway.lifecycle.watch_info(),
            gateway.config.clone(),
            CancellationToken::new(),
        );
        let (client_end, node_end) = memory::pair(NonZeroUsize::new(16).unwrap());
        let (made, contexts) = oneshot::channel();
        let serving = tokio::spawn(async move {
            server
                .serve(node_end, move |peer| {
                    let connection = Connection::owner(peer);
                    drop(made.send(connection.acting_as(Principal::Owner)));
                    connection
                })
                .await
                .unwrap();
        });
        let client = NodeClient::over(
            client_end,
            ClientInfo {
                name: "labels-handler-test".to_owned(),
                version: "0".to_owned(),
            },
            &ClientConfig::default(),
        )
        .await
        .unwrap();
        Self {
            node,
            guarded,
            connection: contexts.await.unwrap(),
            client,
            serving,
            _temporary: temporary,
        }
    }

    async fn close(self) {
        drop(self.connection);
        drop(self.client);
        self.serving.await.unwrap();
        self.node.stop().await.unwrap();
    }
}

fn record(workspace: WorkspaceId) -> RegistryRecord {
    RegistryRecord {
        descriptor: WorkspaceDescriptor {
            id: workspace,
            project: ProjectId::random(),
            path: "/fixture/labels".into(),
            title: None,
            automation: WorkspaceAutomation::Allowed,
            pin: WorkspacePin::Unpinned,
            backing: WorkspaceBacking::Local,
            lifecycle: WorkspaceLifecycle::Active,
            visibility: WorkspaceVisibility::Foreground,
            created_at: DateTime::UNIX_EPOCH,
            revision: RegistryRevision::ZERO,
        },
        recovery: None,
    }
}

fn calls(workspace: WorkspaceId) -> [Call; 5] {
    [
        ListWorkspaceLabels::default().into_call(),
        AssignWorkspaceLabel {
            workspace,
            label: Label {
                name: "Ready".parse().unwrap(),
                color: LabelColor::Violet,
            },
            assignment: Assignment::Assigned,
        }
        .into_call(),
        UpdateWorkspaceLabel {
            name: "Ready".parse().unwrap(),
            new_name: None,
            color: Some(LabelColor::Sky),
        }
        .into_call(),
        InspectWorkspaceLabelDelete {
            name: "Ready".parse().unwrap(),
        }
        .into_call(),
        DeleteWorkspaceLabel {
            name: "Ready".parse().unwrap(),
        }
        .into_call(),
    ]
}

#[tokio::test]
async fn workspace_labels_guarded_handlers_keep_scopes_person_only_and_real_store_results() {
    let fixture = Fixture::open().await;
    let workspace = WorkspaceId::random();
    let store = fixture.guarded.0.sessions.store();
    store
        .put_registry_workspace(record(workspace), None)
        .await
        .unwrap();
    let readonly = fixture.connection.acting_as(Principal::Device {
        id: DeviceId::random(),
        scopes: vec![Scope::Read],
    });
    let agent = fixture.connection.acting_as(Principal::Agent {
        node: fixture.node.info().id,
        session: SessionId::random(),
        scopes: vec![Scope::Read, Scope::Operate],
    });
    for call in calls(workspace) {
        let permitted = call.method().scope() == Some(Scope::Read);
        let result = vessel_proto::dispatch(&*fixture.guarded, &readonly, call.clone()).await;
        if permitted {
            assert!(matches!(result, Outcome::Ok(_)));
        } else {
            assert!(matches!(result, Outcome::Err(error) if error.code == ErrorCode::Forbidden));
        }
        assert!(matches!(
            vessel_proto::dispatch(&*fixture.guarded, &agent, call).await,
            Outcome::Err(error) if error.code == ErrorCode::Forbidden
        ));
    }
    assert!(store.workspace_label_catalog().await.unwrap().is_empty());
    for call in calls(workspace) {
        let Outcome::Ok(result) =
            vessel_proto::dispatch(&*fixture.guarded, &fixture.connection, call).await
        else {
            panic!("the person's owner call must reach the label handler");
        };
        if let Reply::InspectWorkspaceLabelDelete(inspected) = &result {
            assert_eq!(inspected.affected.get(), 1);
        }
        if let Reply::DeleteWorkspaceLabel(deleted) = &result {
            assert_eq!(deleted.affected.get(), 1);
        }
    }
    assert!(store.workspace_label_catalog().await.unwrap().is_empty());
    assert!(store.workspace_registry().await.unwrap().labels.is_empty());
    drop(readonly);
    drop(agent);
    fixture.close().await;
}

#[tokio::test]
async fn workspace_labels_delegated_and_exhausted_observation_does_not_hold_a_wait() {
    let fixture = Fixture::open().await;
    let service = &fixture.guarded.0;
    let initial = service
        .list_workspace_labels(&fixture.connection, ListWorkspaceLabels::default())
        .await
        .unwrap();
    let delegated = fixture.connection.acting_as(Principal::Delegated {
        device: DeviceId::random(),
        scopes: vec![Scope::Read],
        via: NodeId::random(),
        origin: Origin {
            node: NodeId::random(),
            by: OriginBy::Owner,
            scopes: vec![Scope::Read],
        },
    });
    tokio::time::pause();
    let before = tokio::time::Instant::now();
    let request = ListWorkspaceLabels {
        cursor: Some(initial.head),
        read: CatalogRead::Observe,
    };
    assert_eq!(
        service
            .list_workspace_labels(&delegated, request)
            .await
            .unwrap()
            .head,
        initial.head
    );
    assert_eq!(tokio::time::Instant::now(), before);
    let here = service.links.node();
    let shares = Waiter::of(fixture.connection.principal(), here).shares(here);
    let held: Vec<_> = (0..crate::waits::PER_ORIGIN)
        .map(|_| {
            let mut wait = Some(MAX_WAIT_SECS);
            service
                .wait_slot(&fixture.connection, shares.clone(), &mut wait)
                .unwrap()
        })
        .collect();
    assert_eq!(
        service
            .list_workspace_labels(&fixture.connection, request)
            .await
            .unwrap()
            .head,
        initial.head
    );
    assert_eq!(tokio::time::Instant::now(), before);
    drop(held);
    drop(delegated);
    fixture.close().await;
}
