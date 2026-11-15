//! List and inspect selectors make report-id/session combinations unrepresentable.
use super::{
    ReportEntries, ReportSourceFailures, UsageAttemptFailure, UsageReportId, UsageSourceId,
};
use crate::{
    SessionId,
    limits::{ExternalLookup, LimitRefresh},
    usage::CostVisibility,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A person may list a machine; an agent may list only its own local session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UsageReportScope {
    /// All accounts known to this machine's registered sources.
    Machine,
    /// Accounts associated with the session's immutable launch and current provider.
    Session {
        /// Existing conversation identity.
        session: SessionId,
    },
}

/// Report selection is typed even while its method awaits protocol assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct ListUsageReports {
    /// Exactly one source of account association.
    pub scope: UsageReportScope,
    /// Cached reads never discover logins or contact sources/vendors.
    pub refresh: LimitRefresh,
}

/// Exact opaque report lookup shares the list's account resolver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct InspectUsageReport {
    /// Identity obtained from report discovery, never a human label.
    pub id: UsageReportId,
    /// An explicit refresh additionally requires Operate.
    pub refresh: LimitRefresh,
}

/// Discovery failure is independent of successful accounts from other sources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UsageSourceFailure {
    /// Registered source whose previous accounts may be retained.
    pub source: UsageSourceId,
    /// Safe failure category, never helper input or raw stderr.
    pub failure: UsageAttemptFailure,
}

/// A complete cached/fetched observation; no billing-completeness claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UsageReports {
    /// Query completion time, distinct from each account's successful observation.
    pub at: DateTime<Utc>,
    /// Persisted saved-login permission on the destination machine.
    pub policy: ExternalLookup,
    /// Money and Operate-only details are absent for a Read-only caller.
    pub cost_visibility: CostVisibility,
    /// Cache-only disclosure and effective permission; never credential discovery.
    #[serde(default)]
    pub sources: super::ReportSourceDescriptors,
    /// Every known report in stable source/account order; no silent truncation.
    pub reports: ReportEntries,
    /// Independent failed discoveries, even if no account was previously known.
    pub source_failures: ReportSourceFailures,
}

/// Inspect distinguishes a cache miss, disabled lookup and an exact matching report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UsageReportLookup {
    /// Existing report; monetary fields follow this caller's current scopes.
    Found {
        /// Exact matching account report.
        report: Box<super::SourceReportEntry>,
        /// Current caller's monetary visibility.
        cost_visibility: CostVisibility,
    },
    /// Same account resolver established that this exact report does not exist.
    NotFound {
        /// Refused opaque identity.
        id: UsageReportId,
    },
    /// Policy is off; no discovery or vendor request was attempted.
    Off,
}

impl UsageReports {
    /// The response's monetary label and every nested success are projected together.
    #[must_use]
    pub fn for_visibility(mut self, visibility: CostVisibility) -> Self {
        self.reports.redact(visibility);
        self.cost_visibility = visibility;
        self
    }
}
impl UsageReportLookup {
    /// Inspect follows the identical visibility policy as listing and retained reports.
    #[must_use]
    pub fn for_visibility(self, visibility: CostVisibility) -> Self {
        match self {
            Self::Found { report, .. } => Self::Found {
                report: Box::new((*report).for_visibility(visibility)),
                cost_visibility: visibility,
            },
            Self::NotFound { id } => Self::NotFound { id },
            Self::Off => Self::Off,
        }
    }
}
