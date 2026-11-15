//! Reference-shaped synthetic fixtures; no real sign-in or vendor call.
use super::*;
use tokio_util::sync::CancellationToken;
use vessel_proto::{
    HarnessKind,
    limits::{ExtraUsage, LimitPeriod, LimitScope, LimitSlot},
};

fn now() -> DateTime<Utc> {
    "2026-06-01T12:00:00Z".parse().unwrap()
}
#[test]
fn claude_preserves_reported_zero_and_reconciles_legacy_scope() {
    let result = claude::parse(vessel_testkit::limits::CLAUDE_USAGE, now()).unwrap();
    let expected: LimitSnapshot = serde_json::from_str(r#"{
      "at":"2026-06-01T12:00:00Z","plan":null,"windows":[
        {"id":"five_hour","period":{"kind":"five_hour"},"slot":"only","scope":{"kind":"general"},"used":11,"resets_at":"2026-06-01T21:00:00Z"},
        {"id":"weekly","period":{"kind":"weekly"},"slot":"only","scope":{"kind":"general"},"used":1,"resets_at":"2026-06-04T00:00:00Z"},
        {"id":"weekly_model_opus","period":{"kind":"weekly"},"slot":"only","scope":{"kind":"model","id":"opus","name":"Opus"},"used":0,"resets_at":"2026-06-04T00:00:00Z"}
      ],"credits":null,"extra_usage":"disabled","allowed":null,"limit_reached":null,"incomplete":false
    }"#).unwrap();
    assert_eq!(result, expected);
}
#[test]
fn claude_preserves_dimensions_verbatim_ids_unicode_and_partial_sections() {
    let body = br#"{"five_hour":{"utilization":100},"limits":[
      {"kind":"weekly_scoped","percent":0,"scope":{"model":{"id":"fable-pro","display_name":"Fable"}}},
      {"kind":"weekly_scoped","percent":30,"scope":{"model":{"id":"fable_pro","display_name":"Fable"}}},
      {"kind":"weekly_scoped","percent":40,"scope":{"surface":{"display_name":"Fable"}}},
      {"kind":"future_scoped"},42],"extra_usage":{"is_enabled":true}}"#;
    let result = claude::parse(body, now()).unwrap();
    assert_eq!(
        result
            .windows
            .iter()
            .map(|window| (
                window.id.as_str(),
                window.used.map(vessel_proto::limits::LimitPercent::get)
            ))
            .collect::<Vec<_>>(),
        vec![
            ("five_hour", Some(100.0)),
            ("weekly_model_fable-pro", Some(0.0)),
            ("weekly_model_fable_pro", Some(30.0)),
            ("weekly_surface_fable", Some(40.0))
        ]
    );
    assert!(result.incomplete);
    assert_eq!(result.extra_usage, Some(ExtraUsage::Enabled));
    let unicode = r#"{"limits":[{"kind":"weekly_scoped","percent":1,"scope":{"model":{"display_name":"모델"}}},{"kind":"weekly_scoped","percent":2,"scope":{"model":{"display_name":"다름"}}}]}"#;
    let result = claude::parse(unicode.as_bytes(), now()).unwrap();
    assert_eq!(
        result
            .windows
            .iter()
            .map(|window| window.id.as_str())
            .collect::<Vec<_>>(),
        vec!["weekly_model_모델", "weekly_model_다름"]
    );
}
#[test]
fn codex_weekly_identity_follows_duration_not_slot_and_unknown_is_unknown() {
    let secondary = codex::parse(vessel_testkit::limits::CODEX_WEEKLY_ONLY, now()).unwrap();
    let primary = codex::parse(br#"{"rate_limit":{"primary_window":{"used_percent":100,"limit_window_seconds":604800,"reset_at":1700100000}}}"#,now()).unwrap();
    assert_eq!(
        primary.windows.first().unwrap().id,
        secondary.windows.first().unwrap().id
    );
    assert_eq!(primary.windows.first().unwrap().period, LimitPeriod::Weekly);
    assert_eq!(primary.windows.first().unwrap().slot, LimitSlot::Primary);
    assert_eq!(
        secondary.windows.first().unwrap().slot,
        LimitSlot::Secondary
    );
    let unknown = codex::parse(vessel_testkit::limits::CODEX_UNKNOWN_PERIOD, now()).unwrap();
    let first = unknown.windows.first().unwrap();
    assert_eq!(
        (first.id.as_str(), first.period, first.used.unwrap().get()),
        ("general:unknown_primary", LimitPeriod::Unknown, 0.0)
    );
    assert!(matches!(
        unknown.windows[1].scope,
        LimitScope::Feature { .. }
    ));
}
#[test]
fn codex_keeps_credit_eligibility_and_never_guesses_a_currency() {
    let result = codex::parse(br#"{"plan_type":"plus","rate_limit":{"allowed":true,"limit_reached":true,"primary_window":{"used_percent":100,"limit_window_seconds":18000}},"credits":{"balance":"0","has_credits":true,"unlimited":false}}"#,now()).unwrap();
    let expected: LimitSnapshot = serde_json::from_str(r#"{"at":"2026-06-01T12:00:00Z","plan":"Plus","windows":[{"id":"general:five_hour","period":{"kind":"five_hour"},"slot":"primary","scope":{"kind":"general"},"used":100,"resets_at":null}],"credits":{"balance":0,"has_credits":true,"unlimited":false},"extra_usage":null,"allowed":true,"limit_reached":true,"incomplete":false}"#).unwrap();
    assert_eq!(result, expected);
    assert_eq!(
        codex::parse(br#"{"plan_type":"private future account label"}"#, now())
            .unwrap()
            .plan,
        None
    );
}
#[tokio::test]
async fn only_explicit_recorded_sources_read_synthetic_temporary_logins() {
    let root = tempfile::tempdir().unwrap();
    let homes = HarnessEnv::new(root.path().to_owned());
    let profile = Profile::default_for(HarnessKind::Codex);
    let binding = limits_binding(&LimitEnvironment::new(), &homes, &profile).unwrap();
    let path = root.path().join(".codex");
    std::fs::create_dir(&path).unwrap();
    std::fs::write(path.join("auth.json"),br#"{"tokens":{"access_token":"synthetic-oauth-access-for-offline-fixture","account_id":"synthetic-account-id"}}"#).unwrap();
    let body = root.path().join("response.json");
    std::fs::write(&body, vessel_testkit::limits::CODEX_USAGE).unwrap();
    let source = LimitProbe::Recorded {
        root: root.path().to_owned(),
        body,
        status: 200,
        retry_after: None,
    };
    let response = source
        .read(
            &LimitContext::new(),
            &binding,
            now(),
            &CancellationToken::new(),
        )
        .await;
    assert_eq!(
        response.result.unwrap(),
        codex::parse(vessel_testkit::limits::CODEX_USAGE, now()).unwrap()
    );
    assert!(response.identity.is_some());
    assert_eq!(
        LimitProbe::Disabled
            .read(
                &LimitContext::new(),
                &binding,
                now(),
                &CancellationToken::new()
            )
            .await
            .result,
        Err(LimitFailure::Unsupported)
    );
    // Tests enable fakes: accidental Local can never inspect this or a real home.
    assert_eq!(
        LimitProbe::Local
            .read(
                &LimitContext::new(),
                &binding,
                now(),
                &CancellationToken::new()
            )
            .await
            .result,
        Err(LimitFailure::Unsupported)
    );
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert_eq!(
        source
            .read(&LimitContext::new(), &binding, now(), &cancelled)
            .await
            .result,
        Err(LimitFailure::Unsupported)
    );
}
#[test]
fn keychain_custom_service_keeps_nfc_identity_and_never_changes_accounts() {
    assert_eq!(keychain::service(None), "Claude Code-credentials");
    assert_eq!(
        keychain::service(Some("/tmp/é")),
        keychain::service(Some("/tmp/e\u{301}"))
    );
    let service = keychain::service(Some("/tmp/custom"));
    assert_ne!(service, keychain::service(None));
    assert_eq!(
        keychain::arguments(None, &service),
        vec![
            "find-generic-password".to_owned(),
            "-w".to_owned(),
            "-s".to_owned(),
            service
        ]
    );
    assert_eq!(keychain::account(Some("bad account")), "claude-code-user");
}

#[test]
fn malformed_account_identifiers_never_silently_select_the_default_account() {
    for (source,body) in [
        (LimitLoginStore::Codex,br#"{"tokens":{"access_token":"synthetic-offline-access-token","account_id":"wrong\naccount"}}"#.as_slice()),
        (LimitLoginStore::OpenCode,br#"{"openai":{"type":"oauth","access":"synthetic-offline-access-token","accountId":"wrong\naccount"}}"#.as_slice()),
    ] {
        assert!(matches!(
            credential::decode(body, source),
            Err(credential::CredentialError::Unsafe)
        ));
    }
}

#[tokio::test]
async fn a_json_escaped_login_echo_is_refused_before_any_public_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(".codex");
    std::fs::create_dir(&home).unwrap();
    std::fs::write(
        home.join("auth.json"),
        br#"{"tokens":{"access_token":"synthetic-secret-token-only-for-offline-echo"}}"#,
    )
    .unwrap();
    let body = root.path().join("response.json");
    std::fs::write(&body,br#"{"additional_rate_limits":[{"limit_name":"synthetic-secret-token-only-for-offline-\u0065cho","metered_feature":"fixture","rate_limit":{"primary_window":{"used_percent":0}}}]}"#).unwrap();
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
