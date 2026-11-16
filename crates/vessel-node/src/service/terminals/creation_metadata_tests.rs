//! The legacy resolver cannot turn an independent id into spawn authority.
use super::*;
use crate::{Config, Node};
use std::{path::PathBuf, time::Duration};
use vessel_harness::{HarnessEnv, Lookup};
use vessel_proto::WorkspaceId;
use vessel_store::DataDir;

const DEADLINE: Duration = Duration::from_secs(30);

#[tokio::test]
async fn unreceipted_workspace_placement_refuses_without_a_registry_or_terminal_effect() {
    Box::pin(vessel_testkit::within(DEADLINE, async {
        let temp = tempfile::tempdir().unwrap();
        let mut config = Config::new(
            DataDir::new(temp.path().join("data")),
            HarnessEnv::new(temp.path().join("user")),
        );
        config.bind = None;
        config.harness_lookup = Lookup::new(Vec::<PathBuf>::new());
        let node = Node::start(config).await.unwrap();
        let service = &node.secure.service.0;
        let store = service.sessions.store();
        let before = store.workspace_registry().await.unwrap();
        let workspace = WorkspaceId::random();
        let error = service
            .resolve_terminal_place(&Principal::Owner, &TerminalPlace::Workspace { workspace })
            .await;
        assert!(matches!(error, Err(TerminalsError::Unsupported(_))));
        assert!(service.terminals.list(|_| true).is_empty());
        assert_eq!(store.workspace_registry().await.unwrap(), before);
        assert!(store.registry_workspace(workspace).await.unwrap().is_none());
        node.stop().await.unwrap();
    }))
    .await
    .unwrap();
}

#[test]
fn new_terminal_metadata_is_not_forwarded_using_only_the_delivering_hops_version() {
    use vessel_proto::{DeviceId, NodeId, Origin, OriginBy, Scope};
    let delegated = Principal::delegated(
        DeviceId::random(),
        &Scope::ALL,
        Origin {
            node: NodeId::random(),
            by: OriginBy::Owner,
            scopes: Scope::ALL.to_vec(),
        },
        NodeId::random(),
    );
    let info = |place, kind| {
        TerminalInfo::new(
            TerminalId::random(),
            place,
            "/fixture".into(),
            kind,
            Utc::now(),
        )
    };
    let old_place = TerminalPlace::Session {
        session: vessel_proto::SessionId::random(),
    };
    let shell = TerminalKind::Shell {
        program: "fixture-shell".into(),
    };
    let legacy = info(old_place.clone(), shell.clone());
    assert!(metadata_route_readable(&delegated, &legacy));
    for terminal in [
        info(
            TerminalPlace::Workspace {
                workspace: WorkspaceId::random(),
            },
            shell,
        ),
        info(
            old_place,
            TerminalKind::Profile {
                profile: vessel_proto::terminal_profiles::Id::parse("configured").unwrap(),
                revision: vessel_proto::terminal_profiles::ProfileRevision::INITIAL,
            },
        ),
    ] {
        assert!(metadata_route_readable(&Principal::Owner, &terminal));
        assert!(!metadata_route_readable(&delegated, &terminal));
    }
}
