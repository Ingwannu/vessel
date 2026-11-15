//! Unassigned labels stay durable but cannot enter through existing wire methods.
//! Missing update targets prove the refusal precedes lookup as well as writes/spawns.

use crate::support::{Fixture, structured_spec, within};
use vessel_proto::automation::{
    AfterRun, Cadence, CatchUp, CreatePrompt, CreateSchedule, EverySeconds, ListPrompts,
    ListSchedules, ScheduleDefinition, ScheduleTarget, UpdatePrompt, UpdateSchedule,
};
use vessel_proto::{
    Call, CreateSession, ErrorCode, ImportSession, Request, SavedPromptId, ScheduleId, SessionId,
    SessionInfo, SessionLabels, SessionRef, SessionSpec,
};
use vessel_rpc::CallError;

mod raw;
use vessel_store::{Store, StoreConfig};

fn labels(text: &str) -> SessionLabels {
    serde_json::from_str(text).unwrap()
}

fn definition(spec: SessionSpec) -> ScheduleDefinition {
    ScheduleDefinition {
        name: None,
        cadence: Cadence::Every {
            seconds: EverySeconds::try_from(EverySeconds::MIN).unwrap(),
            anchor: None,
        },
        target: ScheduleTarget::NewSession {
            spec: Box::new(spec),
            after: AfterRun::default(),
        },
        prompt: "No work before assignment".parse().unwrap(),
        catch_up: CatchUp::default(),
        max_runs: None,
        ends_at: None,
    }
}

fn labelled_requests(spec: SessionSpec) -> Vec<Call> {
    vec![
        CreateSession { spec: spec.clone() }.into_call(),
        CreatePrompt {
            name: "unassigned".parse().unwrap(),
            text: "No write".parse().unwrap(),
            project: None,
            session: Some(Box::new(spec.clone())),
        }
        .into_call(),
        UpdatePrompt {
            prompt: SavedPromptId::random(),
            revision: 1,
            name: "unassigned".parse().unwrap(),
            text: "No write".parse().unwrap(),
            session: Some(Box::new(spec.clone())),
        }
        .into_call(),
        CreateSchedule {
            definition: definition(spec.clone()),
            paused: true,
        }
        .into_call(),
        UpdateSchedule {
            schedule: ScheduleId::random(),
            revision: 1,
            definition: definition(spec),
        }
        .into_call(),
    ]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_labelled_request_kind_is_refused_before_work_at_current_protocol() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        assert_eq!(client.protocol(), vessel_proto::PROTOCOL_VERSION);
        let mut raw = raw::Raw::connect(&fixture, &node).await;
        let spec = structured_spec(&fixture, None).with_labels(labels(r#"{"env":"prod"}"#));
        for call in labelled_requests(spec) {
            let method = call.method();
            let error = raw.refusal(call).await;
            assert_eq!(error.code, ErrorCode::Unsupported, "{method:?}");
            assert_eq!(
                error.message,
                CallError::UnassignedProtocol { method }.to_string(),
                "{method:?}"
            );
        }
        assert!(client.sessions().await.unwrap().is_empty());
        assert!(
            client
                .call(ListPrompts { project: None })
                .await
                .unwrap()
                .prompts
                .is_empty()
        );
        assert!(
            client
                .call(ListSchedules {})
                .await
                .unwrap()
                .schedules
                .is_empty()
        );
        raw.close().await;
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_starting_import_refuses_before_export_while_storage_keeps_hidden_labels() {
    within(async {
        let fixture = Fixture::new();
        let dir = fixture.data_dir();
        dir.prepare().unwrap();
        let store = Store::open(&dir, StoreConfig::default())
            .await
            .unwrap()
            .store;
        let initial = labels(r#"{"phase":"initial"}"#);
        let current = labels(r#"{"phase":"current"}"#);
        let mut info = SessionInfo::new(
            SessionId::random(),
            structured_spec(&fixture, None).with_labels(initial.clone()),
            chrono::DateTime::UNIX_EPOCH,
        );
        info.labels = current.clone();
        store.create_session(info.clone()).await.unwrap();
        store.close().await;
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let seen = client.session(info.id).await.unwrap();
        assert!(seen.labels.is_empty() && seen.spec.labels.is_empty());
        let destination = fixture.user_home().join("unassigned-import-target");
        let request = ImportSession {
            from: SessionRef {
                node: node.info().id,
                session: info.id,
            },
            profile: None,
            cwd: Some(destination.clone()),
            start: true,
        };
        let error = client.import_session(request).await.unwrap_err();
        assert_eq!(error.code(), Some(ErrorCode::Unsupported));
        assert!(
            error.to_string().contains("no assigned protocol"),
            "{error}"
        );
        assert!(!destination.exists());
        assert_eq!(client.sessions().await.unwrap().len(), 1);
        node.stop().await.unwrap();
        let store = Store::open(&dir, StoreConfig::default())
            .await
            .unwrap()
            .store;
        let kept = store.session(info.id).await.unwrap().unwrap();
        assert_eq!(kept.labels, current);
        assert_eq!(kept.spec.labels, initial);
        store.close().await;
    })
    .await;
}
