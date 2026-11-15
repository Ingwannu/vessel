//! The actual assembled guarded service, without assigning the pending protocol.
use crate::dictation::TestConnection as Link;
use crate::{Config, Node, principal::Principal};
use vessel_proto::{
    ErrorCode, Handler as _, Scope,
    dictation::{
        CancelDictation, ChunkDictation, ChunkSeq, DictationChange, DictationPcm,
        DictationProvider, FinishDictation, NextDictation, Pcm16Mono, SINCE, SampleRate,
        SecretReference, SetDictation, StartDictation,
    },
    settings::{GetNodeSettings, SetNodeSettings},
};

const TEST_LIMIT: std::time::Duration = std::time::Duration::from_secs(60);

fn config(root: &std::path::Path) -> Config {
    let mut config = Config::new(
        vessel_store::DataDir::new(root.join("data")),
        vessel_harness::HarnessEnv::new(root.join("home")),
    );
    config.bind = None;
    config.name = Some("dictation-fixture".to_owned());
    config.harness_lookup = vessel_harness::Lookup::new(std::iter::empty::<std::path::PathBuf>());
    config
}

#[tokio::test]
async fn hidden_get_and_legacy_set_skip_named_presence_reads_on_the_actual_node_service() {
    Box::pin(vessel_testkit::within(TEST_LIMIT, async {
        let root = tempfile::tempdir().unwrap();
        let config = config(root.path());
        std::fs::create_dir_all(config.data_dir.root()).unwrap();
        let name = "HIDDEN_SELECTION"
            .parse::<vessel_proto::SecretName>()
            .unwrap();
        config
            .data_dir
            .write_settings(&SetNodeSettings {
                dictation: Some(SetDictation {
                    provider: Some(DictationChange::Set(DictationProvider::Endpoint)),
                    key: Some(DictationChange::Set(SecretReference::from(name))),
                    ..SetDictation::default()
                }),
                ..SetNodeSettings::default()
            })
            .unwrap();
        let files = crate::fleet::FleetFiles::new(&config.data_dir);
        let node = Node::start(config).await.unwrap();
        std::fs::create_dir_all(files.secrets().parent().unwrap()).unwrap();
        std::fs::write(files.secrets(), "not = [valid secret file").unwrap();
        let link = Link::new().await;
        let read = node
            .secure
            .service
            .node_settings(&link.context, GetNodeSettings {})
            .await;
        let changed = node
            .secure
            .service
            .set_node_settings(
                &link.context,
                SetNodeSettings {
                    park_after: Some(vessel_proto::parking::ParkAfter::Never),
                    ..SetNodeSettings::default()
                },
            )
            .await;
        // Derive expected exposure from the feature constant: assignment later
        // changes this into the supported read's typed store refusal case.
        if SINCE.is_some_and(|required| link.context.protocol() >= required) {
            assert_eq!(read.unwrap_err().code, ErrorCode::Internal);
            assert_eq!(changed.unwrap_err().code, ErrorCode::Internal);
        } else {
            assert!(read.unwrap().dictation.is_none());
            assert!(changed.unwrap().dictation.is_none());
        }
        link.close().await;
        node.stop().await.unwrap();
    }))
    .await
    .unwrap();
}

#[tokio::test]
async fn all_five_actual_guarded_handlers_refuse_read_only_devices_and_agents() {
    Box::pin(vessel_testkit::within(TEST_LIMIT, async {
        let root = tempfile::tempdir().unwrap();
        let node = Node::start(config(root.path())).await.unwrap();
        let link = Link::new().await;
        let viewer = link.context.acting_as(Principal::Device {
            id: vessel_proto::DeviceId::random(),
            scopes: vec![Scope::Read],
        });
        let agent = link.context.acting_as(Principal::Agent {
            node: vessel_proto::NodeId::random(),
            session: vessel_proto::SessionId::random(),
            scopes: vec![Scope::Read, Scope::Operate],
        });
        for connection in [&viewer, &agent] {
            let id = "scope-refusal"
                .parse::<vessel_proto::dictation::DictationId>()
                .unwrap();
            let guard = &node.secure.service;
            assert_eq!(
                guard
                    .start_dictation(
                        connection,
                        StartDictation {
                            id: id.clone(),
                            format: Pcm16Mono::new(SampleRate::MIN)
                        }
                    )
                    .await
                    .unwrap_err()
                    .code,
                ErrorCode::Forbidden
            );
            assert_eq!(
                guard
                    .chunk_dictation(
                        connection,
                        ChunkDictation {
                            id: id.clone(),
                            seq: ChunkSeq::new(0),
                            pcm: DictationPcm::new(1000_i16.to_le_bytes().to_vec()).unwrap()
                        }
                    )
                    .await
                    .unwrap_err()
                    .code,
                ErrorCode::Forbidden
            );
            assert_eq!(
                guard
                    .finish_dictation(
                        connection,
                        FinishDictation {
                            id: id.clone(),
                            final_seq: ChunkSeq::new(0)
                        }
                    )
                    .await
                    .unwrap_err()
                    .code,
                ErrorCode::Forbidden
            );
            assert_eq!(
                guard
                    .next_dictation(connection, NextDictation { id: id.clone() })
                    .await
                    .unwrap_err()
                    .code,
                ErrorCode::Forbidden
            );
            assert_eq!(
                guard
                    .cancel_dictation(connection, CancelDictation { id })
                    .await
                    .unwrap_err()
                    .code,
                ErrorCode::Forbidden
            );
        }
        link.close().await;
        node.stop().await.unwrap();
    }))
    .await
    .unwrap();
}
