//! Real service/connection owners and loopback HTTP, without protocol assignment.
use super::*;
use crate::dictation::{endpoint::tests::Server, registry::test_fixture::Link};
use axum::{Router, routing::post};
use vessel_proto::{
    ErrorCode, ErrorDetail, Method, Scope,
    dictation::{
        ChunkSeq, DictationChange, DictationPcm, DictationProvider, KeyPresence, Pcm16Mono,
        SampleRate, SecretReference, SetDictation,
    },
    settings::SetNodeSettings,
};

const TEST_LIMIT: std::time::Duration = std::time::Duration::from_secs(60);

struct Fixture {
    _root: tempfile::TempDir,
    settings: Arc<Settings>,
    files: crate::fleet::FleetFiles,
    name: vessel_proto::SecretName,
    server: Server,
    stop: CancellationToken,
    work: TaskTracker,
}
impl Fixture {
    async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let data = vessel_store::DataDir::new(root.path());
        let settings = crate::settings::test_fixture::open(data.clone());
        let files = crate::fleet::FleetFiles::new(&data);
        let name = "TRANSCRIPTION".parse().unwrap();
        crate::fleet::set_secret(&files, &name, "synthetic-service-key").unwrap();
        let server = Server::start(Router::new().route(
            "/v1/audio/transcriptions",
            post(|| async { "{\"text\":\"recognized\"}" }),
        ))
        .await;
        Self {
            _root: root,
            settings,
            files,
            name,
            server,
            stop: CancellationToken::new(),
            work: TaskTracker::new(),
        }
    }
    fn patch(&self) -> SetNodeSettings {
        SetNodeSettings {
            dictation: Some(SetDictation {
                provider: Some(DictationChange::Set(DictationProvider::Endpoint)),
                endpoint_base: Some(DictationChange::Set(self.server.base())),
                key: Some(DictationChange::Set(SecretReference::from(
                    self.name.clone(),
                ))),
                ..SetDictation::default()
            }),
            ..SetNodeSettings::default()
        }
    }
    fn service(&self) -> Service {
        Service::new(Arc::clone(&self.settings), &self.stop, &self.work)
    }
    async fn finish(self) {
        self.stop.cancel();
        let _first_close = self.work.close();
        self.work.wait().await;
        self.server.finish().await;
    }
}

#[tokio::test]
async fn deletion_between_selection_and_save_keeps_only_a_reference_and_start_reports_key_absent() {
    vessel_testkit::within(TEST_LIMIT, async {
        let mut f = Fixture::new().await;
        let gate = Arc::get_mut(&mut f.settings).unwrap().hold_writes();
        let service = f.service();
        let mut saving = Box::pin(f.settings.change_dictation(f.patch(), service.reader()));
        assert!(futures_util::poll!(saving.as_mut()).is_pending());
        gate.entered().await;
        assert!(crate::fleet::delete_secret(&f.files, &f.name).unwrap());
        gate.release();
        let reply = saving.await.unwrap();
        assert_eq!(reply.dictation.as_ref().unwrap().key, KeyPresence::Present);
        let read = f
            .settings
            .current_dictation(service.reader())
            .await
            .unwrap();
        assert_eq!(read.dictation.as_ref().unwrap().key, KeyPresence::Absent);
        let rendered = serde_json::to_string(&read).unwrap();
        assert!(!rendered.contains(f.name.as_str()));
        assert!(!rendered.contains("synthetic-service-key"));
        assert_eq!(
            f.settings.dictation_snapshot().await.key_name(),
            Some(&f.name)
        );
        let link = Link::new().await;
        let context = link.context.dictation_context().unwrap();
        let error = service
            .start(
                &context,
                StartDictation {
                    id: "deleted-key".parse().unwrap(),
                    format: Pcm16Mono::new(SampleRate::MIN),
                },
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::Conflict);
        assert_eq!(
            error.detail,
            Some(ErrorDetail::Dictation {
                reason: DictationReason::KeyAbsent
            })
        );
        link.close().await;
        f.finish().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn the_product_service_transcribes_a_tail_and_delivers_partial_then_final_with_real_connection_ownership()
 {
    vessel_testkit::within(TEST_LIMIT, async {
        let f = Fixture::new().await;
        let service = f.service();
        let _configured = f
            .settings
            .change_dictation(f.patch(), service.reader())
            .await
            .unwrap();
        let link = Link::new().await;
        let context = link.context.dictation_context().unwrap();
        let id: vessel_proto::dictation::DictationId = "service-flow".parse().unwrap();
        let _ready = service
            .start(
                &context,
                StartDictation {
                    id: id.clone(),
                    format: Pcm16Mono::new(SampleRate::MIN),
                },
            )
            .await
            .unwrap();
        let ack = service
            .chunk(
                &context,
                ChunkDictation {
                    id: id.clone(),
                    seq: ChunkSeq::new(0),
                    pcm: DictationPcm::new(1000_i16.to_le_bytes().repeat(2)).unwrap(),
                },
            )
            .await
            .unwrap();
        assert_eq!(ack.through, Some(ChunkSeq::new(0)));
        let _accepted = service
            .finish(
                &context,
                FinishDictation {
                    id: id.clone(),
                    final_seq: ChunkSeq::new(0),
                },
            )
            .await
            .unwrap();
        match service
            .next(&context, NextDictation { id: id.clone() })
            .await
            .unwrap()
        {
            DictationUpdate::Partial { text } => assert!(text.as_str() == "recognized"),
            DictationUpdate::Final { .. }
            | DictationUpdate::Failed { .. }
            | DictationUpdate::Unknown => panic!("expected a partial"),
        }
        match service
            .next(&context, NextDictation { id: id.clone() })
            .await
            .unwrap()
        {
            DictationUpdate::Final { text } => assert!(text.as_str() == "recognized"),
            DictationUpdate::Partial { .. }
            | DictationUpdate::Failed { .. }
            | DictationUpdate::Unknown => panic!("expected the final"),
        }
        assert_eq!(
            service
                .next(&context, NextDictation { id })
                .await
                .unwrap_err()
                .code,
            ErrorCode::NotFound
        );
        link.close().await;
        f.finish().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn every_registered_operation_needs_operate_and_agent_contexts_are_refused_before_work() {
    vessel_testkit::within(TEST_LIMIT, async {
        let link = Link::new().await;
        let viewer = crate::principal::Principal::Device {
            id: vessel_proto::DeviceId::random(),
            scopes: vec![Scope::Read],
        };
        let operator = crate::principal::Principal::Device {
            id: vessel_proto::DeviceId::random(),
            scopes: vec![Scope::Operate],
        };
        for method in [
            Method::StartDictation,
            Method::ChunkDictation,
            Method::FinishDictation,
            Method::NextDictation,
            Method::CancelDictation,
        ] {
            assert_eq!(
                viewer.permit(method).unwrap_err().code,
                ErrorCode::Forbidden
            );
            operator.permit(method).unwrap();
        }
        let agent = link.context.acting_as(crate::principal::Principal::Agent {
            node: vessel_proto::NodeId::random(),
            session: vessel_proto::SessionId::random(),
            scopes: vec![Scope::Read, Scope::Operate],
        });
        let Err(error) = agent.dictation_context() else {
            panic!("agent recording context was admitted");
        };
        assert_eq!(error.code, ErrorCode::Forbidden);
        assert!(!link.context.life().closed().is_cancelled());
        link.close().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn the_node_work_tracker_joins_service_retirement_while_a_real_context_is_still_retained() {
    vessel_testkit::within(TEST_LIMIT, async {
        let f = Fixture::new().await;
        let service = f.service();
        let _configured = f
            .settings
            .change_dictation(f.patch(), service.reader())
            .await
            .unwrap();
        let link = Link::new().await;
        let context = link.context.dictation_context().unwrap();
        let id = "node-stop".parse().unwrap();
        let _ready = service
            .start(
                &context,
                StartDictation {
                    id,
                    format: Pcm16Mono::new(SampleRate::MIN),
                },
            )
            .await
            .unwrap();
        let mut waiting = Box::pin(service.next(
            &context,
            NextDictation {
                id: "node-stop".parse().unwrap(),
            },
        ));
        assert!(futures_util::poll!(waiting.as_mut()).is_pending());
        f.stop.cancel();
        match waiting.await.unwrap() {
            DictationUpdate::Failed { reason } => assert_eq!(reason, DictationReason::Cancelled),
            DictationUpdate::Partial { .. }
            | DictationUpdate::Final { .. }
            | DictationUpdate::Unknown => panic!("node stop produced text"),
        }
        let _first_close = f.work.close();
        f.work.wait().await;
        assert!(!link.context.life().closed().is_cancelled());
        link.close().await;
        f.finish().await;
    })
    .await
    .unwrap();
}
