//! Whole responses and isolated homes, including safe failures and cooldown.
use super::*;
use tokio_util::sync::CancellationToken;
use vessel_proto::{HarnessKind, limits::LimitFailure};
fn now() -> DateTime<Utc> {
    "2026-06-01T12:00:00Z".parse().unwrap()
}
#[tokio::test]
async fn expired_rejected_rate_limited_changed_and_rotated_are_separate() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(".claude");
    std::fs::create_dir(&home).unwrap();
    let auth = home.join(".credentials.json");
    let body = root.path().join("body.json");
    std::fs::write(
        &auth,
        br#"{"claudeAiOauth":{"accessToken":"synthetic-first-login-fixture-token","expiresAt":1}}"#,
    )
    .unwrap();
    std::fs::write(&body, vessel_testkit::limits::CLAUDE_USAGE).unwrap();
    let binding = limits_binding(
        &LimitEnvironment::new(),
        &HarnessEnv::new(root.path().to_owned()),
        &Profile::default_for(HarnessKind::ClaudeCode),
    )
    .unwrap();
    let source = |status, retry_after| LimitProbe::Recorded {
        root: root.path().to_owned(),
        body: body.clone(),
        status,
        retry_after,
    };
    let expired = probe(source(200, None), &binding).await;
    assert_eq!(expired.result, Err(LimitFailure::Expired));
    std::fs::write(
        &auth,
        br#"{"claudeAiOauth":{"accessToken":"synthetic-first-login-fixture-token"}}"#,
    )
    .unwrap();
    let good = probe(source(200, None), &binding).await;
    assert!(good.result.is_ok());
    assert_eq!(good.identity, expired.identity);
    let rejected = probe(source(403, None), &binding).await;
    assert_eq!(rejected.result, Err(LimitFailure::Rejected));
    assert_eq!(rejected.identity, good.identity);
    let limited = probe(source(429, Some("600".to_owned())), &binding).await;
    assert_eq!(limited.result, Err(LimitFailure::RateLimited));
    assert_eq!(
        limited.retry_at,
        Some("2026-06-01T12:10:00Z".parse().unwrap())
    );
    std::fs::write(&body, b"<html>not JSON</html>").unwrap();
    let changed = probe(source(200, None), &binding).await;
    assert_eq!(changed.result, Err(LimitFailure::ChangedResponse));
    assert_eq!(changed.identity, good.identity);
    std::fs::write(
        &auth,
        br#"{"claudeAiOauth":{"accessToken":"synthetic-second-login-fixture-token"}}"#,
    )
    .unwrap();
    let rotated = probe(source(200, None), &binding).await;
    assert_ne!(rotated.identity, good.identity);
    std::fs::remove_file(&auth).unwrap();
    let absent = probe(source(200, None), &binding).await;
    assert_eq!(absent.result, Err(LimitFailure::NoLogin));
    assert_eq!(absent.identity, None);
}
#[test]
fn configured_routes_and_relative_homes_fail_without_io() {
    let root = tempfile::tempdir().unwrap();
    let homes = HarnessEnv::new(root.path().to_owned());
    for (variable, value) in [
        ("CLAUDE_CODE_USE_BEDROCK", "true"),
        ("CLAUDE_CODE_USE_VERTEX", "YES"),
        ("ANTHROPIC_BASE_URL", "https://other.invalid"),
    ] {
        let env = LimitEnvironment::from_vars(|name| (name == variable).then(|| value.into()));
        assert_eq!(
            limits_binding(&env, &homes, &Profile::default_for(HarnessKind::ClaudeCode)),
            Err(LimitFailure::DifferentRoute)
        );
    }
    let homes = homes.with_var("CODEX_HOME", "relative".into());
    assert_eq!(
        limits_binding(
            &LimitEnvironment::new(),
            &homes,
            &Profile::default_for(HarnessKind::Codex)
        ),
        Err(LimitFailure::UnsafeLoginStore)
    );
    assert_eq!(
        limits_binding(
            &LimitEnvironment::new(),
            &homes,
            &Profile::default_for(HarnessKind::Gemini)
        ),
        Err(LimitFailure::Unsupported)
    );
}
#[test]
fn recorded_guard_and_bounded_local_read_never_escape_to_another_home() {
    let allowed = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let path = other.path().join("synthetic-auth.json");
    std::fs::write(
        &path,
        br#"{"tokens":{"access_token":"synthetic-only-fixture-access-token"}}"#,
    )
    .unwrap();
    assert!(matches!(
        credential::read(
            &path,
            Some(allowed.path()),
            LimitLoginStore::Codex
        ),
        Err(error) if error.failure() == LimitFailure::UnsafeLoginStore
    ));
    std::fs::write(&path, vec![b' '; 1_048_577]).unwrap();
    assert!(matches!(
        credential::bounded(&path),
        Err(error) if error.failure() == LimitFailure::UnsafeLoginStore
    ));
}
#[tokio::test]
async fn a_vendor_cannot_echo_the_access_token_into_a_display_label() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(".codex");
    std::fs::create_dir(&home).unwrap();
    std::fs::write(
        home.join("auth.json"),
        br#"{"tokens":{"access_token":"synthetic-private-access-for-echo-check"}}"#,
    )
    .unwrap();
    let body = root.path().join("body.json");
    std::fs::write(&body,br#"{"additional_rate_limits":[{"limit_name":"synthetic-private-access-for-echo-check","rate_limit":{"primary_window":{"used_percent":1}}}]}"#).unwrap();
    let binding = limits_binding(
        &LimitEnvironment::new(),
        &HarnessEnv::new(root.path().to_owned()),
        &Profile::default_for(HarnessKind::Codex),
    )
    .unwrap();
    let result = LimitProbe::Recorded {
        root: root.path().to_owned(),
        body,
        status: 200,
        retry_after: None,
    }
    .read(
        &LimitContext::new(),
        &binding,
        now(),
        &CancellationToken::new(),
    )
    .await;
    assert_eq!(result.result, Err(LimitFailure::ChangedResponse));
}

async fn probe(source: LimitProbe, binding: &LimitBinding) -> LimitProbeResult {
    source
        .read(
            &LimitContext::new(),
            binding,
            now(),
            &CancellationToken::new(),
        )
        .await
}
