//! Pending methods are invoked through their real guarded service, never a forged protocol number.
//! The framed handshake supplies the actual Connection; there is no assigned wire/runtime claim.
use super::*;
use vessel_proto::{
    Handler as _,
    usage_reports::{SetUsageSourceLookup, SourceReportState, UsageSourceAvailability},
};
mod harness;
use harness::{Fixture, SERVICE_TEST_BUDGET};
fn cursor(reports: &UsageReports) -> &SourceReportEntry {
    reports
        .reports
        .as_slice()
        .iter()
        .find(|report| report.source().as_str() == "cursor")
        .unwrap()
}
#[tokio::test]
async fn real_service_enforces_consent_refresh_and_redaction_without_read_discovery() {
    tokio::time::timeout(SERVICE_TEST_BUDGET, async {
        let fixture = Fixture::start().await;
        let off = fixture.list(&fixture.reader, LimitRefresh::Cached).await;
        assert_eq!(off.policy, ExternalLookup::Off);
        assert!(off.reports.is_empty());
        assert_eq!(fixture.reads(), 0);
        assert!(
            off.sources
                .as_slice()
                .iter()
                .any(|source| source.id.as_str() == "cursor"
                    && source.availability == UsageSourceAvailability::GlobalOff)
        );
        assert_eq!(
            fixture
                .service()
                .set_usage_source_lookup(
                    &fixture.reader,
                    Fixture::source_request(ExternalLookup::On)
                )
                .await
                .unwrap_err()
                .code,
            ErrorCode::Forbidden
        );
        fixture.allow().await;
        assert_eq!(
            fixture.reads(),
            0,
            "global/source consent must not discover a login"
        );
        let cached = fixture.list(&fixture.reader, LimitRefresh::Cached).await;
        assert!(
            cached
                .reports
                .as_slice()
                .iter()
                .all(|entry| entry.source().as_str() != "cursor")
        );
        assert_eq!(fixture.reads(), 0, "a Read miss must remain a miss");
        assert_eq!(
            fixture
                .service()
                .list_usage_reports(
                    &fixture.reader,
                    ListUsageReports {
                        scope: UsageReportScope::Machine,
                        refresh: LimitRefresh::Refresh
                    }
                )
                .await
                .unwrap_err()
                .code,
            ErrorCode::Forbidden
        );
        assert_eq!(fixture.reads(), 0);
        let current = fixture.list(&fixture.owner, LimitRefresh::Refresh).await;
        let entry = cursor(&current);
        let id = entry.id().clone();
        let SourceReportState::Available { observed } = &entry.state else {
            panic!("real recorded service success")
        };
        let original = observed.clone();
        assert!(observed.report.balances.as_slice()[0].remaining.is_some());
        assert_eq!(observed.report.details.as_slice().len(), 1);
        let reads = fixture.reads();
        assert!(reads > 0);
        let read = fixture.list(&fixture.reader, LimitRefresh::Cached).await;
        let SourceReportState::Available { observed } = &cursor(&read).state else {
            panic!("cached success")
        };
        assert_eq!(read.cost_visibility, CostVisibility::Withheld);
        assert!(observed.report.balances.as_slice()[0].remaining.is_none());
        assert!(observed.report.details.is_empty());
        assert_eq!(fixture.reads(), reads);
        std::fs::write(
            fixture.root.path().join("report.json"),
            b"malformed synthetic source",
        )
        .unwrap();
        let failed = fixture.list(&fixture.owner, LimitRefresh::Refresh).await;
        assert_eq!(cursor(&failed).id(), &id);
        let SourceReportState::Failed {
            retained: Some(retained),
            ..
        } = &cursor(&failed).state
        else {
            panic!("same-binding retained service failure")
        };
        assert_eq!(retained, &original);
        let private = fixture.list(&fixture.reader, LimitRefresh::Cached).await;
        let SourceReportState::Failed {
            retained: Some(retained),
            ..
        } = &cursor(&private).state
        else {
            panic!("retained read projection")
        };
        assert_eq!(retained.at, original.at);
        assert!(retained.report.balances.as_slice()[0].remaining.is_none());
        assert!(retained.report.details.is_empty());
        let reads = fixture.reads();
        let unknown = fixture
            .service()
            .inspect_usage_report(
                &fixture.owner,
                InspectUsageReport {
                    id: "cursor:unknown".parse().unwrap(),
                    refresh: LimitRefresh::Refresh,
                },
            )
            .await
            .unwrap();
        assert!(matches!(unknown, UsageReportLookup::NotFound { .. }));
        assert_eq!(
            fixture.reads(),
            reads,
            "an unknown opaque id must not discover"
        );
        assert_eq!(
            fixture
                .service()
                .inspect_usage_report(
                    &fixture.reader,
                    InspectUsageReport {
                        id,
                        refresh: LimitRefresh::Refresh
                    }
                )
                .await
                .unwrap_err()
                .code,
            ErrorCode::Forbidden
        );
        let _off = fixture
            .service()
            .set_usage_source_lookup(&fixture.owner, Fixture::source_request(ExternalLookup::Off))
            .await
            .unwrap();
        let revoked = fixture.list(&fixture.reader, LimitRefresh::Cached).await;
        assert!(
            revoked
                .reports
                .as_slice()
                .iter()
                .all(|entry| entry.source().as_str() != "cursor")
        );
        assert_eq!(fixture.reads(), reads);
        fixture.finish().await;
    })
    .await
    .unwrap();
}

const REFRESH_SAFETY_BUDGET: std::time::Duration = std::time::Duration::from_secs(95);
#[tokio::test(start_paused = true)]
async fn aggregate_refresh_cannot_hold_a_request_indefinitely() {
    let before = tokio::time::Instant::now();
    let refusal = tokio::time::timeout(
        REFRESH_SAFETY_BUDGET,
        refresh_within_budget(std::future::pending::<Result<(), WireError>>()),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert_eq!(refusal.code, ErrorCode::Unavailable);
    assert!(before.elapsed() >= REPORT_REFRESH_BUDGET);
    assert!(before.elapsed() < REFRESH_SAFETY_BUDGET);
}
