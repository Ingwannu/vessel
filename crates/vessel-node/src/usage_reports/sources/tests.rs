//! Real source owner, recorded vendor body, synthetic credential capability; no live account.
mod cancellation;
mod harness;
use super::*;
use harness::{Fixture, TEST_BUDGET};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use vessel_proto::usage_reports::{
    SourceReportState, UsageDisclosureVersion, UsageSourceAvailability,
};
#[tokio::test]
async fn global_opt_in_source_consent_and_cache_reads_never_read_credentials() {
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&calls);
    let fixture = Fixture::start(move |name| {
        let _prior = seen.fetch_add(1, Ordering::SeqCst);
        (name == "COPILOT_TOKEN").then(|| "synthetic-source-token".into())
    })
    .await;
    let first = fixture.sources.snapshot(&fixture.global).await.unwrap();
    let copilot = first
        .sources
        .as_slice()
        .iter()
        .find(|source| source.id.as_str() == "copilot")
        .unwrap();
    assert_eq!(copilot.availability, UsageSourceAvailability::SourceOff);
    assert!(
        fixture
            .sources
            .refresh_all(&fixture.global)
            .await
            .is_empty()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let allowed = fixture.allow(ExternalLookup::On).await;
    assert_eq!(allowed.availability, UsageSourceAvailability::Enabled);
    assert_eq!(calls.load(Ordering::SeqCst), 0, "consent must not fetch");
    assert!(
        fixture
            .sources
            .snapshot(&fixture.global)
            .await
            .unwrap()
            .reports
            .is_empty()
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "a cached miss must not discover a login"
    );
    assert!(
        fixture
            .sources
            .refresh_all(&fixture.global)
            .await
            .is_empty()
    );
    let successful = fixture.sources.snapshot(&fixture.global).await.unwrap();
    assert_eq!(successful.reports.len(), 1);
    let id = successful.reports[0].id().clone();
    assert!(matches!(
        successful.reports[0].state,
        SourceReportState::Available { .. }
    ));
    let reads = calls.load(Ordering::SeqCst);
    let again = fixture.sources.snapshot(&fixture.global).await.unwrap();
    assert_eq!(again.reports[0], successful.reports[0]);
    assert_eq!(calls.load(Ordering::SeqCst), reads);
    let _off = fixture.allow(ExternalLookup::Off).await;
    assert!(
        !successful.still_current(),
        "revocation must invalidate an already copied observation"
    );
    assert!(
        fixture
            .sources
            .snapshot(&fixture.global)
            .await
            .unwrap()
            .reports
            .is_empty()
    );
    assert!(
        fixture
            .sources
            .inspect(
                &id,
                &fixture.global,
                vessel_proto::limits::LimitRefresh::Refresh
            )
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        reads,
        "inspect misses must not discover credentials"
    );
    fixture.finish().await;
}
#[tokio::test]
async fn exact_disclosure_is_required_and_disabled_or_unknown_sources_cannot_be_enabled() {
    let fixture = Fixture::start(|_name| None).await;
    let mut stale = Fixture::request(ExternalLookup::On);
    stale.disclosure_version = serde_json::from_str::<UsageDisclosureVersion>("2").unwrap();
    assert_eq!(
        fixture
            .sources
            .set(stale, &fixture.global)
            .await
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    let mut unsupported = Fixture::request(ExternalLookup::On);
    unsupported.source = "cursor".parse().unwrap();
    assert_eq!(
        fixture
            .sources
            .set(unsupported, &fixture.global)
            .await
            .unwrap_err()
            .code,
        ErrorCode::Unsupported
    );
    let mut unknown = Fixture::request(ExternalLookup::On);
    unknown.source = "not-registered".parse().unwrap();
    assert_eq!(
        fixture
            .sources
            .set(unknown, &fixture.global)
            .await
            .unwrap_err()
            .code,
        ErrorCode::NotFound
    );
    assert!(fixture.dir().read_usage_sources().unwrap().is_empty());
    fixture.finish().await;
}
#[tokio::test]
async fn same_login_failure_retains_only_its_original_success_and_changed_login_gets_another_id() {
    let changed = Arc::new(AtomicBool::new(false));
    let picked = Arc::clone(&changed);
    let fixture = Fixture::start(move |name| {
        (name == "COPILOT_TOKEN").then(|| {
            if picked.load(Ordering::SeqCst) {
                "fixture-token-B".into()
            } else {
                "fixture-token-A".into()
            }
        })
    })
    .await;
    let _on = fixture.allow(ExternalLookup::On).await;
    assert!(
        fixture
            .sources
            .refresh_all(&fixture.global)
            .await
            .is_empty()
    );
    let first = fixture.sources.snapshot(&fixture.global).await.unwrap();
    let id_a = first.reports[0].id().clone();
    let SourceReportState::Available { observed } = first.reports[0].state.clone() else {
        panic!("available")
    };
    std::fs::write(fixture.body(), b"malformed synthetic body").unwrap();
    assert!(
        fixture
            .sources
            .refresh_all(&fixture.global)
            .await
            .is_empty()
    );
    let failed = fixture.sources.snapshot(&fixture.global).await.unwrap();
    assert_eq!(failed.reports[0].id(), &id_a);
    let SourceReportState::Failed {
        retained: Some(retained),
        ..
    } = &failed.reports[0].state
    else {
        panic!("retained same-login success")
    };
    assert_eq!(retained, &observed);
    changed.store(true, Ordering::SeqCst);
    assert!(
        fixture
            .sources
            .refresh_all(&fixture.global)
            .await
            .is_empty()
    );
    let rebound = fixture.sources.snapshot(&fixture.global).await.unwrap();
    assert_ne!(rebound.reports[0].id(), &id_a);
    assert!(matches!(
        rebound.reports[0].state,
        SourceReportState::Failed { retained: None, .. }
    ));
    assert!(
        fixture
            .sources
            .inspect(
                &id_a,
                &fixture.global,
                vessel_proto::limits::LimitRefresh::Cached
            )
            .await
            .unwrap()
            .is_none()
    );
    fixture.finish().await;
}
#[tokio::test]
async fn dropping_a_consent_rpc_does_not_abandon_its_actor_owned_persistence() {
    let fixture = Fixture::start(|_name| None).await;
    let (reply, response) = oneshot::channel();
    drop(response);
    fixture
        .sources
        .0
        .as_ref()
        .unwrap()
        .try_send(Command::Set {
            request: Fixture::request(ExternalLookup::On),
            global: fixture.global.clone(),
            reply,
        })
        .unwrap();
    let snapshot = tokio::time::timeout(TEST_BUDGET, fixture.sources.snapshot(&fixture.global))
        .await
        .unwrap()
        .unwrap();
    assert!(
        snapshot
            .sources
            .as_slice()
            .iter()
            .any(|source| source.id.as_str() == "copilot"
                && source.availability == UsageSourceAvailability::Enabled)
    );
    assert_eq!(
        fixture.dir().read_usage_sources().unwrap().as_slice().len(),
        1
    );
    fixture.finish().await;
}
