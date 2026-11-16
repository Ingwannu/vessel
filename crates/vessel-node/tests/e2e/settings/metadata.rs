//! Admission and candidate validation both precede settings mutation.

use super::*;
use vessel_client::CallError;
use vessel_proto::settings::{MetadataProvider, MetadataSettings};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_pending_or_invalid_metadata_patch_leaves_the_file_and_settings_unchanged() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        let path = fixture.data_dir().settings_path();
        let before = path.exists().then(|| std::fs::read(&path).unwrap());
        let original = owner.call(GetNodeSettings {}).await.unwrap();
        let metadata = MetadataSettings::new(vec![MetadataProvider {
            route: "missing-metadata-route".parse().unwrap(),
            model: None,
            effort: None,
        }])
        .unwrap();
        let refused = owner
            .call(SetNodeSettings {
                metadata: Some(metadata),
                ..SetNodeSettings::default()
            })
            .await;
        match vessel_proto::GENERATED_NAMES {
            None => assert!(matches!(refused, Err(CallError::UnassignedProtocol { .. }))),
            Some(required) if required > vessel_proto::PROTOCOL_VERSION => {
                assert!(matches!(refused, Err(CallError::NotInProtocol { .. })));
            }
            Some(_) => assert_eq!(code(refused), ErrorCode::InvalidRequest),
        }
        assert_eq!(owner.call(GetNodeSettings {}).await.unwrap(), original);
        assert_eq!(path.exists().then(|| std::fs::read(&path).unwrap()), before);
        node.stop().await.unwrap();
    })
    .await;
}
