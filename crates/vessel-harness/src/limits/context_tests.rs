//! Observable source-boundary backoff and cancelled-reader admission.
use super::*;
use std::{path::Path, time::Duration};
use tokio_util::sync::CancellationToken;
use vessel_proto::{HarnessKind, limits::LimitFailure};
fn now() -> DateTime<Utc> {
    "2026-06-01T12:00:00Z".parse().unwrap()
}
fn codex(root: &Path) -> LimitBinding {
    let home = root.join(".codex");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join("auth.json"),br#"{"tokens":{"access_token":"synthetic-shared-chatgpt-token-for-offline-probe","account_id":"synthetic-first-account"}}"#).unwrap();
    limits_binding(
        &LimitEnvironment::new(),
        &HarnessEnv::new(root.to_owned()),
        &Profile::default_for(HarnessKind::Codex),
    )
    .unwrap()
}
#[tokio::test]
async fn retry_after_is_shared_across_profiles_and_opencode_using_the_same_token() {
    let root = tempfile::tempdir().unwrap();
    let binding = codex(root.path());
    let context = LimitContext::new();
    let body = root.path().join("response.json");
    std::fs::write(&body, vessel_testkit::limits::CODEX_USAGE).unwrap();
    let limited = LimitProbe::Recorded {
        root: root.path().to_owned(),
        body: body.clone(),
        status: 429,
        retry_after: Some("600".to_owned()),
    };
    let stop = CancellationToken::new();
    let first = limited.read(&context, &binding, now(), &stop).await;
    assert_eq!(first.result, Err(LimitFailure::RateLimited));
    assert_eq!(
        first.retry_at,
        Some("2026-06-01T12:10:00Z".parse().unwrap())
    );
    let path = root.path().join(".local/share/opencode");
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(path.join("auth.json"),br#"{"openai":{"type":"oauth","access":"synthetic-shared-chatgpt-token-for-offline-probe","accountId":"synthetic-second-account"}}"#).unwrap();
    let other = limits_binding(
        &LimitEnvironment::new(),
        &HarnessEnv::new(root.path().to_owned()),
        &Profile::default_for(HarnessKind::OpenCode),
    )
    .unwrap();
    let ready = LimitProbe::Recorded {
        root: root.path().to_owned(),
        body,
        status: 200,
        retry_after: None,
    };
    let blocked = ready.read(&context, &other, now(), &stop).await;
    assert_eq!(blocked.result, Err(LimitFailure::RateLimited));
    assert_eq!(blocked.retry_at, first.retry_at);
    assert_ne!(blocked.identity, first.identity);
    let later = ready
        .read(
            &context,
            &other,
            "2026-06-01T12:11:00Z".parse().unwrap(),
            &stop,
        )
        .await;
    assert!(later.result.is_ok());
}
#[test]
fn cancelled_probe_callers_cannot_replace_queued_blocking_reads() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        vessel_testkit::within(Duration::from_secs(30), async {
            let root = tempfile::tempdir().unwrap();
            let binding = codex(root.path());
            let context = LimitContext::new();
            let body = root.path().join("response.json");
            std::fs::write(&body, vessel_testkit::limits::CODEX_USAGE).unwrap();
            let source = LimitProbe::Recorded {
                root: root.path().to_owned(),
                body,
                status: 200,
                retry_after: None,
            };
            let (entered, started) = tokio::sync::oneshot::channel();
            let (release, wait) = std::sync::mpsc::sync_channel(1);
            let blocker = tokio::task::spawn_blocking(move || {
                entered.send(()).unwrap();
                wait.recv().unwrap();
            });
            started.await.unwrap();
            let stopped = CancellationToken::new();
            let clock = now();
            let mut first = Box::pin(source.read(&context, &binding, clock, &stopped));
            let mut second = Box::pin(source.read(&context, &binding, clock, &stopped));
            assert!(futures_util::poll!(&mut first).is_pending());
            assert!(futures_util::poll!(&mut second).is_pending());
            drop(first);
            drop(second);
            let refused = source.read(&context, &binding, clock, &stopped).await;
            assert_eq!(refused.result, Err(LimitFailure::Busy));
            assert_eq!(refused.identity, None);
            release.send(()).unwrap();
            blocker.await.unwrap();
            context.drain().await;
            assert!(
                source
                    .read(&context, &binding, clock, &stopped)
                    .await
                    .result
                    .is_ok()
            );
        })
        .await
        .unwrap();
    });
}
