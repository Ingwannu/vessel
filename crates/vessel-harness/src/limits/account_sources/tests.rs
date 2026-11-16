//! Recorded source fixtures never construct HTTP clients or read the process account.
use super::*;
use std::{
    ffi::OsString,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};
use vessel_proto::usage_reports::UsageProblem;
pub(super) mod support;
use support::{CASE_BUDGET, PhaseWait, Release};
fn access(root: &Path) -> AccountSourceAccess {
    AccountSourceAccess::new(root.to_owned(), |_name| None)
}
fn write(root: &Path, path: &str, body: &[u8]) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}
fn recorded(
    root: &Path,
    source: BuiltinUsageSource,
    access: AccountSourceAccess,
    body: &[u8],
) -> AccountUsageProbe {
    write(root, "recorded-body.json", body);
    AccountUsageProbe::Recorded {
        root: root.to_owned(),
        access,
        source,
        body: root.join("recorded-body.json"),
        status: 200,
        retry_after: None,
        format: RecordedUsageFormat::Vendor,
    }
}
#[tokio::test]
async fn recorded_observation_preserves_same_login_proof_and_rejects_token_echo() {
    vessel_testkit::within(CASE_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            ".grok/auth.json",
            br#"{"https://auth.x.ai::fixture":{"key":"synthetic-source-token"}}"#,
        );
        let probe = recorded(
            root.path(),
            BuiltinUsageSource::Grok,
            access(root.path()),
            br#"{"usage":{"creditUsage":10}}"#,
        );
        let context = LimitContext::new();
        let stop = CancellationToken::new();
        let first = probe
            .read(&context, BuiltinUsageSource::Grok, Utc::now(), &stop)
            .phase("usage phase: probe .read")
            .await;
        assert!(matches!(
            first.result,
            Ok(AccountUsageOutcome::Available(_))
        ));
        assert!(first.identity.is_some());
        let again = probe
            .read(&context, BuiltinUsageSource::Grok, Utc::now(), &stop)
            .phase("usage phase: probe .read")
            .await;
        assert_eq!(first.identity, again.identity);
        write(
            root.path(),
            ".grok/auth.json",
            br#"{"access_token":"different-synthetic-token"}"#,
        );
        let changed = probe
            .read(&context, BuiltinUsageSource::Grok, Utc::now(), &stop)
            .phase("usage phase: probe .read")
            .await;
        assert_ne!(first.identity, changed.identity);
        let copy = AccountSourceAccess::new(root.path().to_owned(), |name| {
            (name == "COPILOT_TOKEN").then(|| OsString::from("credential-must-not-become-plan"))
        });
        let echo = recorded(
            root.path(),
            BuiltinUsageSource::Copilot,
            copy,
            br#"{"copilot_plan":"credential-must-not-become-plan"}"#,
        );
        assert_eq!(
            echo.read(&context, BuiltinUsageSource::Copilot, Utc::now(), &stop)
                .phase("usage assertion: echo.read")
                .await
                .result,
            Err(LimitFailure::ChangedResponse)
        );
        context.drain().phase("usage phase: context.drain").await;
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn expiry_cancellation_and_fixed_origin_refusals_happen_before_body_access() {
    vessel_testkit::within(CASE_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            ".kimi-code/credentials/kimi-code.json",
            br#"{"access_token":"synthetic-expired","expires_at":"100"}"#,
        );
        let probe = recorded(
            root.path(),
            BuiltinUsageSource::Kimi,
            access(root.path()),
            b"must never decode",
        );
        let context = LimitContext::new();
        let stop = CancellationToken::new();
        assert_eq!(
            probe
                .read(
                    &context,
                    BuiltinUsageSource::Kimi,
                    DateTime::from_timestamp(200, 0).unwrap(),
                    &stop
                )
                .phase("usage assertion: probe .read")
                .await
                .result,
            Err(LimitFailure::Expired)
        );
        stop.cancel();
        assert_eq!(
            probe
                .read(&context, BuiltinUsageSource::Kimi, Utc::now(), &stop)
                .phase("usage assertion: probe .read")
                .await
                .result,
            Err(LimitFailure::Unsupported)
        );
        let access = AccountSourceAccess::new(root.path().to_owned(), |name| match name {
            "MINIMAX_API_KEY" => Some("fixture-only".into()),
            "MINIMAX_BASE_URL" => Some("https://foreign.invalid".into()),
            _ => None,
        });
        let probe = recorded(
            root.path(),
            BuiltinUsageSource::MiniMax,
            access,
            b"must never decode",
        );
        assert_eq!(
            probe
                .read(
                    &context,
                    BuiltinUsageSource::MiniMax,
                    Utc::now(),
                    &CancellationToken::new()
                )
                .phase("usage assertion: probe .read")
                .await
                .result,
            Err(LimitFailure::DifferentRoute)
        );
        context.drain().phase("usage phase: context.drain").await;
    })
    .await
    .unwrap();
}
#[test]
fn capabilities_are_deferred_and_each_stored_shape_has_an_exact_source() {
    let root = tempfile::tempdir().unwrap();
    let stop = CancellationToken::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&calls);
    let deferred = AccountSourceAccess::new(root.path().to_owned(), move |name| {
        let _prior = seen.fetch_add(1, Ordering::SeqCst);
        (name == "GLM_API_KEY").then(|| "fixture-zai".into())
    });
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        credential::read(&deferred, BuiltinUsageSource::Zai, Some(root.path()), &stop)
            .unwrap()
            .token
            .as_str(),
        "fixture-zai"
    );
    assert!(calls.load(Ordering::SeqCst) > 0);
    write(root.path(),".config/gh/hosts.yml",b"github.example.invalid:\n  oauth_token: do_not_send\ngithub.com:\n  oauth_token: fixture_github\n");
    assert_eq!(
        credential::read(
            &access(root.path()),
            BuiltinUsageSource::Copilot,
            Some(root.path()),
            &stop
        )
        .unwrap()
        .token
        .as_str(),
        "fixture_github"
    );
    write(root.path(),".local/share/opencode/auth.json",br#"{"opencode-go":{"type":"api","key":"fixture-go"},"openai":{"type":"oauth","access":"wrong-source"}}"#);
    assert_eq!(
        credential::read(
            &access(root.path()),
            BuiltinUsageSource::OpenCodeGo,
            Some(root.path()),
            &stop
        )
        .unwrap()
        .token
        .as_str(),
        "fixture-go"
    );
    write(
        root.path(),
        ".mmx/config.json",
        br#"{"api_key":"fixture-cn","region":"cn"}"#,
    );
    let mini = credential::read(
        &access(root.path()),
        BuiltinUsageSource::MiniMax,
        Some(root.path()),
        &stop,
    )
    .unwrap();
    assert!(matches!(mini.region, credential::Region::China));
    write(
        root.path(),
        ".config/cursor/auth.json",
        br#"{"accessToken":"fixture-cursor"}"#,
    );
    assert_eq!(
        credential::read(
            &access(root.path()),
            BuiltinUsageSource::Cursor,
            Some(root.path()),
            &stop
        )
        .unwrap()
        .token
        .as_str(),
        "fixture-cursor"
    );
}
#[tokio::test]
async fn no_plan_is_distinct_from_a_transport_error_and_retry_after_is_preserved() {
    vessel_testkit::within(CASE_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let access = AccountSourceAccess::new(root.path().to_owned(), |name| {
            (name == "ZAI_API_KEY").then(|| "fixture-key".into())
        });
        let mut probe = recorded(
            root.path(),
            BuiltinUsageSource::Zai,
            access,
            br#"{"data":[]}"#,
        );
        let context = LimitContext::new();
        let stop = CancellationToken::new();
        assert!(matches!(
            probe
                .read(&context, BuiltinUsageSource::Zai, Utc::now(), &stop)
                .phase("usage assertion: probe .read")
                .await
                .result,
            Ok(AccountUsageOutcome::Unavailable(
                UsageProblem::NoQuota { .. }
            ))
        ));
        if let AccountUsageProbe::Recorded {
            status,
            retry_after,
            ..
        } = &mut probe
        {
            *status = 429;
            *retry_after = Some("120".to_owned());
        }
        let at = DateTime::UNIX_EPOCH;
        let limited = probe
            .read(&context, BuiltinUsageSource::Zai, at, &stop)
            .phase("usage phase: probe .read")
            .await;
        assert_eq!(limited.result, Err(LimitFailure::RateLimited));
        assert_eq!(limited.retry_at, Some(at + chrono::TimeDelta::seconds(120)));
        context.drain().phase("usage phase: context.drain").await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn report_fixture_preserves_forecasts_and_still_refuses_credential_echo() {
    vessel_testkit::within(CASE_BUDGET, async {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        ".config/cursor/auth.json",
        br#"{"accessToken":"fixture-cursor-secret"}"#,
    );
    let mut probe = recorded(
        root.path(),
        BuiltinUsageSource::Cursor,
        access(root.path()),
        br#"{"plan":"Synthetic","windows":[{"id":"forecast","label":"Forecast","short_label":null,"used":90,"remaining":7,"resets_at":"2030-01-01T00:00:00Z","runs_out_at":"2029-12-31T23:00:00Z","shortfall":17,"tone":"danger"}],"balances":[],"details":[]}"#,
    );
    let AccountUsageProbe::Recorded { format, .. } = &mut probe else {
        panic!("recorded only")
    };
    *format = RecordedUsageFormat::Report;
    let context = LimitContext::new();
    let stop = CancellationToken::new();
    let first = probe
        .read(&context, BuiltinUsageSource::Cursor, Utc::now(), &stop).phase("usage phase: probe .read")
        .await;
    let Ok(AccountUsageOutcome::Available(report)) = &first.result else {
        panic!("recorded source report")
    };
    assert_eq!(
        report.windows.as_slice()[0]
            .shortfall
            .unwrap()
            .get()
            .to_string(),
        "17"
    );
    assert!(
        report.windows.as_slice()[0].runs_out_at.unwrap()
            < report.windows.as_slice()[0].resets_at.unwrap()
    );

    write(
        root.path(),
        "recorded-body.json",
        br#"{"plan":"fixture-cursor-secret", "windows":[],"balances":[],"details":[]}"#,
    );
    let echoed = probe
        .read(&context, BuiltinUsageSource::Cursor, Utc::now(), &stop).phase("usage phase: probe .read")
        .await;
    assert_eq!(echoed.result, Err(LimitFailure::ChangedResponse));
    assert_eq!(first.identity, echoed.identity);
    context.drain().phase("usage phase: context.drain").await;

    }).await.unwrap();
}

mod lifetimes;
