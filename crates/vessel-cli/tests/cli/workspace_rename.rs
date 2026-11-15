//! Real protocol25 metadata acknowledgements across all consuming output modes.
use crate::support::{Fixture, within};
use std::path::Path;
use vessel_proto::{
    WorkspaceId,
    registry::{GetWorkspace, ListWorkspaces},
};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Receipt {
    workspace_id: WorkspaceId,
    title: Option<String>,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_workspace_rename_outputs_only_actual_applied_titles_and_null_reset() {
    within(async {
        let fixture = Fixture::start_with_registry(record).await;
        let owner = fixture.owner().await;
        let list = owner.call(ListWorkspaces::default()).await.unwrap();
        assert_eq!(list.len(), 1);
        let id = list.first().unwrap().id;
        for (title, flags) in [
            ("table name", vec![]),
            ("no header", vec!["--no-headers"]),
            ("json name", vec!["--json"]),
            ("yaml name", vec!["-o", "yaml"]),
            ("quiet name", vec!["--quiet"]),
        ] {
            let id_text = id.to_string();
            let mut args = vec!["workspace", "rename", &id_text, title];
            args.extend(flags);
            let applied = fixture.vessel(&args, b"").await;
            assert!(applied.ok, "{}", applied.stderr);
            let actual = owner.call(GetWorkspace { workspace: id }).await.unwrap();
            assert_eq!(
                actual.title.as_ref().map(ToString::to_string).as_deref(),
                Some(title)
            );
            match title {
                "json name" => checked(&applied.stdout, id, Some(title)),
                "yaml name" => checked(
                    applied.stdout.strip_prefix("---\n").unwrap(),
                    id,
                    Some(title),
                ),
                "quiet name" => assert_eq!(applied.stdout, format!("{id}\n")),
                "no header" => assert_eq!(applied.stdout, format!("{id}  {title}\n")),
                _ => assert_eq!(
                    applied.stdout,
                    format!("WORKSPACE ID  TITLE\n{id}  {title}\n")
                ),
            }
        }
        let reset = fixture
            .vessel(
                &["workspace", "rename", &id.to_string(), "--reset", "--json"],
                b"",
            )
            .await;
        assert!(reset.ok, "{}", reset.stderr);
        checked(&reset.stdout, id, None);
        assert!(
            owner
                .call(GetWorkspace { workspace: id })
                .await
                .unwrap()
                .title
                .is_none()
        );
        fixture.stop().await;
    })
    .await;
}
fn checked(text: &str, id: WorkspaceId, title: Option<&str>) {
    let receipt: Receipt = serde_json::from_str(text).unwrap();
    assert_eq!(receipt.workspace_id, id);
    assert_eq!(receipt.title.as_deref(), title);
}
fn record(path: &Path) -> vessel_store::RegistryRecord {
    use vessel_proto::registry::{
        RegistryRevision, WorkspaceAutomation, WorkspaceBacking, WorkspaceDescriptor,
        WorkspaceLifecycle, WorkspacePin, WorkspaceVisibility,
    };
    vessel_store::RegistryRecord {
        descriptor: WorkspaceDescriptor {
            id: WorkspaceId::random(),
            project: vessel_proto::ProjectId::random(),
            path: path.to_owned(),
            title: None,
            automation: WorkspaceAutomation::Allowed,
            pin: WorkspacePin::Unpinned,
            backing: WorkspaceBacking::Local,
            lifecycle: WorkspaceLifecycle::Active,
            visibility: WorkspaceVisibility::Foreground,
            created_at: chrono::DateTime::UNIX_EPOCH,
            revision: RegistryRevision::ZERO,
        },
        recovery: None,
    }
}
