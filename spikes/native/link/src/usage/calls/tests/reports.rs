//! A node with account reports (protocol 26 and later): the inventory reads
//! `usage.reports` and `usage.inspect` only, never the saved-login methods.
use super::*;
use vessel_proto::usage_reports::{UsageReportId, UsageReportLookup, UsageReports};

fn reports() -> UsageReports {
    UsageReports {
        at: chrono::DateTime::UNIX_EPOCH,
        policy: vessel_proto::limits::ExternalLookup::On,
        cost_visibility: CostVisibility::Permitted,
        sources: vessel_proto::usage_reports::ReportSourceDescriptors::default(),
        reports: vessel_proto::usage_reports::ReportEntries::default(),
        source_failures: vessel_proto::usage_reports::ReportSourceFailures::default(),
    }
}

#[tokio::test]
async fn reports_are_read_cached_and_refreshed_only_by_refresh_all() {
    let modes = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&modes);
    let peer = peer::connect(move |call| {
        let Call::ListUsageReports(ask) = call else {
            panic!("a reports node is asked for usage.reports only");
        };
        seen.lock().unwrap().push(ask.refresh);
        Outcome::Ok(Reply::ListUsageReports(reports()))
    })
    .await;
    let cached = inventory(&peer.client, &InventoryRefresh::Cached)
        .await
        .unwrap();
    assert_eq!(cached.source_reports(), Some(&reports()));
    let all = inventory(&peer.client, &InventoryRefresh::All)
        .await
        .unwrap();
    assert!(matches!(
        all,
        Inventory::Sources {
            refresh_problem: None,
            ..
        }
    ));
    assert_eq!(
        *modes.lock().unwrap(),
        [LimitRefresh::Cached, LimitRefresh::Refresh]
    );
}

#[tokio::test]
async fn one_report_refresh_inspects_it_then_reads_the_list_cached() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&order);
    let peer = peer::connect(move |call| {
        if let Call::InspectUsageReport(ask) = call {
            seen.lock().unwrap().push(("inspect", ask.refresh));
            return Outcome::Ok(Reply::InspectUsageReport(UsageReportLookup::NotFound {
                id: ask.id,
            }));
        }
        let Call::ListUsageReports(ask) = call else {
            panic!("a reports node is asked for report methods only");
        };
        seen.lock().unwrap().push(("list", ask.refresh));
        Outcome::Ok(Reply::ListUsageReports(reports()))
    })
    .await;
    let id: UsageReportId = "cursor:fixture".parse().unwrap();
    let answer = inventory(&peer.client, &InventoryRefresh::Report(id))
        .await
        .unwrap();
    // A report the node no longer knows is the refresh's problem; the list
    // is still read, cached.
    assert!(matches!(
        answer,
        Inventory::Sources {
            refresh_problem: Some(RequestFailure::Refused(ErrorCode::NotFound)),
            ..
        }
    ));
    assert_eq!(
        *order.lock().unwrap(),
        [
            ("inspect", LimitRefresh::Refresh),
            ("list", LimitRefresh::Cached)
        ]
    );
}

#[tokio::test]
async fn an_older_node_falls_back_without_being_asked_for_reports() {
    let asked = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&asked);
    let peer = peer::connect_at(before_reports(), move |call| {
        let reports_call = matches!(
            call,
            Call::ListUsageReports(_) | Call::InspectUsageReport(_)
        );
        seen.lock().unwrap().push(reports_call);
        if matches!(call, Call::ListProfiles(_)) {
            return Outcome::Ok(Reply::ListProfiles(vec![profile("first")]));
        }
        let Call::GetUsageLimits(ask) = call else {
            panic!("an older node is asked for its saved-login methods only");
        };
        let LimitsScope::Account { account } = ask.scope else {
            panic!("account only");
        };
        Outcome::Ok(Reply::GetUsageLimits(limits(account)))
    })
    .await;
    let answer = inventory(&peer.client, &InventoryRefresh::Cached)
        .await
        .unwrap();
    assert!(matches!(answer, Inventory::Saved { .. }));
    // The client refuses the report method itself: nothing reaches the node.
    assert!(!asked.lock().unwrap().is_empty());
    assert!(
        !asked
            .lock()
            .unwrap()
            .iter()
            .any(|reports_call| *reports_call)
    );
}
