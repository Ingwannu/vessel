//! A real resumed create keeps its original claim after preparation has settled.

use super::*;
use vessel_proto::{HarnessSessionId, SessionId};

const STEP: Duration = Duration::from_secs(20);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resumed_create_refuses_a_duplicate_between_preparation_and_registration() {
    vessel_testkit::within(SAFETY_NET, Box::pin(async {
        let fixture = Fixture::new();
        let spec = SessionSpec::new(HarnessKind::ClaudeCode, SessionMode::Structured {
            model: None, resume: Some(HarnessSessionId::new("prepared-conversation")), options: Vec::new(),
        }).with_cwd(fixture.work());
        let runtime = Arc::new(fixture.runtime);
        let first_id = SessionId::random();
        let arriving = runtime.before_registration.arm();
        let starting = { let runtime = Arc::clone(&runtime); let spec = spec.clone();
            tokio::spawn(async move { runtime.create_with(spec, LaunchOptions { id: Some(first_id), ..LaunchOptions::default() }).await }) };
        let held = tokio::time::timeout(STEP, arriving).await.unwrap().unwrap();
        assert!(runtime.pids().is_empty(), "the first create is prepared but has not spawned");
        let duplicate = tokio::time::timeout(STEP, runtime.create(spec)).await.unwrap();
        assert!(matches!(duplicate, Err(crate::SessionError::ConversationTaken(id)) if id == first_id));
        drop(held);
        let Created { handle, mut durable, .. } = tokio::time::timeout(STEP, starting).await.unwrap().unwrap().unwrap();
        assert_eq!(handle.id(), first_id);
        assert!(runtime.get(first_id).is_some());
        let draining = tokio::spawn(async move { while durable.recv().await.is_some() {} });
        tokio::time::timeout(STEP, runtime.shutdown()).await.unwrap();
        tokio::time::timeout(STEP, draining).await.unwrap().unwrap();
    })).await.unwrap();
}
