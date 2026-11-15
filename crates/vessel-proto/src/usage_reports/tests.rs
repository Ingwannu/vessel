//! Contract tests use synthetic metadata; no credential or vendor endpoint exists.
use super::*;
use crate::{
    conversation::FiniteNumber,
    limits::{LimitName, LimitPercent},
    usage::CostVisibility,
};
use chrono::DateTime;
fn name(text: &str) -> LimitName {
    text.parse().unwrap()
}
fn window(id: &str) -> UsageWindow {
    UsageWindow {
        id: id.parse().unwrap(),
        label: name("Session"),
        short_label: None,
        summary: UsageSummary::Included,
        used: Some(LimitPercent::reported(FiniteNumber::new(31.0).unwrap())),
        remaining: None,
        resets_at: None,
        runs_out_at: None,
        shortfall: None,
        tone: None,
    }
}
fn observed() -> SuccessfulUsageReport {
    SuccessfulUsageReport {
        at: DateTime::UNIX_EPOCH,
        report: AvailableUsageReport {
            completeness: UsageCompleteness::Complete,
            plan: Some(name("Example plan")),
            windows: vec![window("session")].try_into().unwrap(),
            balances: vec![UsageBalance {
                id: "credits".parse().unwrap(),
                label: name("Credits"),
                used: Some(FiniteNumber::new(123.25).unwrap()),
                remaining: Some(FiniteNumber::new(6.0).unwrap()),
                limit: Some(FiniteNumber::new(200.0).unwrap()),
                unit: UsageBalanceUnit::Credits,
                resets_at: None,
                tone: None,
            }]
            .try_into()
            .unwrap(),
            details: vec![
                UsageDetail {
                    id: "public".parse().unwrap(),
                    label: name("Status"),
                    value: name("Active"),
                    access: UsageDetailAccess::Read,
                    tone: None,
                },
                UsageDetail {
                    id: "private".parse().unwrap(),
                    label: name("Paid use"),
                    value: name("Example amount"),
                    access: UsageDetailAccess::Operate,
                    tone: None,
                },
            ]
            .try_into()
            .unwrap(),
        },
    }
}
fn entry(id: &str, state: SourceReportState) -> SourceReportEntry {
    SourceReportEntry::new(
        id.parse().unwrap(),
        name("Example source"),
        None,
        None,
        Some(name("Same human label")),
        state,
    )
}
#[test]
fn duplicate_ids_are_refused_but_equal_display_labels_are_not_identities() {
    let first = entry("example:first", SourceReportState::NotRequested);
    let second = entry("example:second", SourceReportState::NotRequested);
    assert!(ReportEntries::try_from(vec![first.clone(), second]).is_ok());
    assert!(matches!(
        ReportEntries::try_from(vec![first.clone(), first]),
        Err(InvalidUsageCollection::Duplicate {
            kind: UsageCollection::Reports
        })
    ));
    assert!(matches!(
        ReportWindows::try_from(vec![window("same"), window("same")]),
        Err(InvalidUsageCollection::Duplicate {
            kind: UsageCollection::Windows
        })
    ));
}
#[test]
fn every_collection_checks_its_own_bound_before_accepting_values() {
    let maximum: Vec<_> = (0..MAX_USAGE_WINDOWS)
        .map(|index| window(&format!("window-{index}")))
        .collect();
    assert!(ReportWindows::try_from(maximum.clone()).is_ok());
    let mut excess = maximum.clone();
    excess.push(window("one-too-many"));
    assert!(matches!(
        ReportWindows::try_from(excess),
        Err(InvalidUsageCollection::TooMany {
            kind: UsageCollection::Windows
        })
    ));
    let encoded = serde_json::to_string(&maximum).unwrap();
    let prefix = encoded.strip_suffix(']').unwrap();
    let malformed = format!("{prefix},{{\"body_must_not_decode\": [malformed excess payload");
    let error = serde_json::from_str::<ReportWindows>(&malformed)
        .unwrap_err()
        .to_string();
    assert!(error.contains(&MAX_USAGE_WINDOWS.to_string()), "{error}");
    assert!(
        !error.contains("EOF"),
        "the excess body must not be decoded: {error}"
    );
}
#[test]
fn all_current_and_retained_success_paths_redact_money_and_private_details() {
    let success = observed();
    let states = [
        SourceReportState::Available {
            observed: success.clone(),
        },
        SourceReportState::Refreshing {
            origin: UsageRefreshOrigin::Available,
            started_at: Some(DateTime::UNIX_EPOCH),
            retained: Some(success.clone()),
        },
        SourceReportState::Unavailable {
            attempted_at: Some(DateTime::UNIX_EPOCH),
            retry_at: None,
            problem: UsageProblem::Login {
                reason: crate::limits::LimitFailure::Expired,
            },
            retained: Some(success.clone()),
            logins: ReportLoginFailures::default(),
        },
        SourceReportState::Failed {
            attempted_at: Some(DateTime::UNIX_EPOCH),
            retry_at: None,
            reason: UsageFetchFailure::TimedOut,
            retained: Some(success),
            logins: ReportLoginFailures::default(),
        },
    ];
    for state in states {
        let public = entry("example:account", state).for_visibility(CostVisibility::Withheld);
        let value = match public.state {
            SourceReportState::Available { observed } => observed,
            SourceReportState::Refreshing { retained, .. }
            | SourceReportState::Unavailable { retained, .. }
            | SourceReportState::Failed { retained, .. } => retained.unwrap(),
            SourceReportState::NotRequested => panic!("fixture always has a success"),
        };
        assert_eq!(value.at, DateTime::UNIX_EPOCH);
        let balance = value.report.balances.as_slice().first().unwrap();
        assert!(balance.used.is_none() && balance.remaining.is_none() && balance.limit.is_none());
        assert_eq!(value.report.details.as_slice().len(), 1);
        assert_eq!(
            value.report.details.as_slice().first().unwrap().id.as_str(),
            "public"
        );
        assert_eq!(
            value
                .report
                .windows
                .as_slice()
                .first()
                .unwrap()
                .used
                .unwrap()
                .get()
                .to_string(),
            "31"
        );
    }
}
#[test]
fn list_and_inspect_use_one_visibility_projection_and_preserve_exact_lookup_id() {
    let report = entry(
        "example:account",
        SourceReportState::Available {
            observed: observed(),
        },
    );
    let inspected = UsageReportLookup::Found {
        report: Box::new(report.clone()),
        cost_visibility: CostVisibility::Permitted,
    }
    .for_visibility(CostVisibility::Withheld);
    let listed = UsageReports {
        at: DateTime::UNIX_EPOCH,
        policy: crate::limits::ExternalLookup::On,
        cost_visibility: CostVisibility::Permitted,
        reports: vec![report].try_into().unwrap(),
        sources: ReportSourceDescriptors::default(),
        source_failures: ReportSourceFailures::default(),
    }
    .for_visibility(CostVisibility::Withheld);
    let UsageReportLookup::Found {
        report,
        cost_visibility,
    } = inspected
    else {
        panic!("found")
    };
    assert_eq!(cost_visibility, CostVisibility::Withheld);
    assert_eq!(listed.cost_visibility, CostVisibility::Withheld);
    assert_eq!(listed.reports.as_slice(), &[*report]);
    let missing: UsageReportId = "example:missing".parse().unwrap();
    let result = UsageReportLookup::NotFound {
        id: missing.clone(),
    }
    .for_visibility(CostVisibility::Withheld);
    assert_eq!(result, UsageReportLookup::NotFound { id: missing });
}
#[test]
fn selected_report_ids_and_session_selectors_are_separate_wire_shapes() {
    let list = ListUsageReports {
        scope: UsageReportScope::Machine,
        refresh: crate::limits::LimitRefresh::Cached,
    };
    let inspect = InspectUsageReport {
        id: "example:one".parse().unwrap(),
        refresh: crate::limits::LimitRefresh::Refresh,
    };
    assert_eq!(
        serde_json::from_str::<ListUsageReports>(&serde_json::to_string(&list).unwrap()).unwrap(),
        list
    );
    assert_eq!(
        serde_json::from_str::<InspectUsageReport>(&serde_json::to_string(&inspect).unwrap())
            .unwrap(),
        inspect
    );
}

#[test]
fn a_complete_collection_checks_encoded_bytes_without_silently_dropping_accounts() {
    let mut success = observed();
    success.report.windows = (0..MAX_USAGE_WINDOWS)
        .map(|index| window(&format!("window-{index}")))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    let reports = (0..MAX_USAGE_REPORTS)
        .map(|index| {
            entry(
                &format!("example:account-{index}"),
                SourceReportState::Available {
                    observed: success.clone(),
                },
            )
        })
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    let large = UsageReports {
        at: DateTime::UNIX_EPOCH,
        policy: crate::limits::ExternalLookup::On,
        cost_visibility: CostVisibility::Permitted,
        reports,
        sources: ReportSourceDescriptors::default(),
        source_failures: ReportSourceFailures::default(),
    };
    assert!(matches!(
        large.validate_budget(),
        Err(UsageReportBudgetError::TooLarge)
    ));
    assert_eq!(large.reports.as_slice().len(), MAX_USAGE_REPORTS);
    let small = UsageReports {
        at: DateTime::UNIX_EPOCH,
        policy: crate::limits::ExternalLookup::Off,
        cost_visibility: CostVisibility::Withheld,
        reports: ReportEntries::default(),
        sources: ReportSourceDescriptors::default(),
        source_failures: ReportSourceFailures::default(),
    };
    assert!(small.validate_budget().unwrap().get() < MAX_USAGE_RESPONSE_BYTES);
}
#[test]
fn unknown_selector_fields_cannot_smuggle_report_ids_into_a_session_query() {
    let query = ListUsageReports {
        scope: UsageReportScope::Session {
            session: crate::SessionId::random(),
        },
        refresh: crate::limits::LimitRefresh::Cached,
    };
    let mut wire = serde_json::to_value(query).unwrap();
    assert!(
        wire.as_object_mut()
            .unwrap()
            .insert("report_ids".to_owned(), serde_json::json!(["example:one"]))
            .is_none()
    );
    assert!(serde_json::from_value::<ListUsageReports>(wire).is_err());
    let mut inspect = serde_json::to_value(InspectUsageReport {
        id: "example:one".parse().unwrap(),
        refresh: crate::limits::LimitRefresh::Cached,
    })
    .unwrap();
    assert!(
        inspect
            .as_object_mut()
            .unwrap()
            .insert(
                "session".to_owned(),
                serde_json::json!(crate::SessionId::random())
            )
            .is_none()
    );
    assert!(serde_json::from_value::<InspectUsageReport>(inspect).is_err());
}

#[test]
fn explicit_source_and_account_ids_cannot_disagree_with_the_opaque_handle() {
    let original = entry("example:account", SourceReportState::NotRequested);
    assert_eq!(original.source().as_str(), "example");
    assert_eq!(original.account().as_str(), "account");
    let wire = serde_json::to_value(&original).unwrap();
    assert_eq!(
        serde_json::from_value::<SourceReportEntry>(wire.clone()).unwrap(),
        original
    );
    let mut wrong_source = wire.clone();
    wrong_source["source"] = serde_json::json!("other");
    let mut wrong_account = wire;
    wrong_account["account"] = serde_json::json!("other");
    assert!(serde_json::from_value::<SourceReportEntry>(wrong_source).is_err());
    assert!(serde_json::from_value::<SourceReportEntry>(wrong_account).is_err());
}

#[test]
fn source_methods_stay_pending_with_explicit_authority_and_unary_shape() {
    use crate::{
        Scope,
        rpc::{Method, MethodShape},
    };
    for (method, scope) in [
        (Method::ListUsageReports, Scope::Read),
        (Method::InspectUsageReport, Scope::Read),
        (Method::SetUsageSourceLookup, Scope::Admin),
    ] {
        assert_eq!(method.since(), None);
        assert_eq!(method.scope(), Some(scope));
        assert_eq!(method.shape(), MethodShape::Unary);
    }
}

#[test]
fn refresh_origin_roundtrip_preserves_current_success_and_omission_is_safe() {
    let state = SourceReportState::Refreshing {
        origin: UsageRefreshOrigin::Available,
        started_at: None,
        retained: Some(observed()),
    };
    let mut wire = serde_json::to_value(&state).unwrap();
    assert_eq!(
        serde_json::from_value::<SourceReportState>(wire.clone()).unwrap(),
        state
    );
    assert!(wire.as_object_mut().unwrap().remove("origin").is_some());
    assert!(matches!(
        serde_json::from_value::<SourceReportState>(wire).unwrap(),
        SourceReportState::Refreshing {
            origin: UsageRefreshOrigin::Unavailable,
            ..
        }
    ));
}
