//! Current attempts own errors; only successes own values and observation times.
use super::{
    AvailableUsageReport, BuiltinUsageSource, ReportLoginFailures, UsageAccountKey, UsageReportId,
    UsageSourceId,
};
use crate::{
    HarnessRef,
    limits::{LimitFailure, LimitName},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A provider reported no currently usable allowance; no raw response text crosses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum NoQuotaReason {
    /// No active coding subscription.
    NoActivePlan,
    /// This product exposes no allowance for the selected login.
    NoAllowance,
    /// A reported subscription is inactive.
    Inactive,
}

/// Safe unavailable categories reuse the reviewed saved-login failure taxonomy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UsageProblem {
    /// Saved-login safety, route, cooldown, or provider-support refusal.
    Login {
        /// Sanitized existing category; no credential path or error body.
        reason: LimitFailure,
    },
    /// The source explicitly reported no active quota.
    NoQuota {
        /// Closed reason instead of arbitrary provider text.
        reason: NoQuotaReason,
    },
}

/// Transport/runtime faults have no free-text or credential-bearing diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum UsageFetchFailure {
    /// Source deadline elapsed; completion must still release its admission.
    TimedOut,
    /// Fixed vendor origin could not be reached.
    Unreachable,
    /// Vendor data could not be represented by the validated schema.
    ChangedResponse,
    /// Source disappeared or its generation changed during the call.
    SourceRemoved,
    /// Existing bounded work must finish before another lookup starts.
    Busy,
    /// Trusted plugin helper refused or ended; raw stderr is not public data.
    HelperUnavailable,
}

/// Several login attempts may fail before one account receives a successful report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UsageAttemptFailure {
    /// Login or subscription facts prevented a report.
    Unavailable {
        /// Safe failure category.
        problem: UsageProblem,
    },
    /// Runtime/fetch failed.
    Error {
        /// Safe transport/schema category.
        reason: UsageFetchFailure,
    },
}

/// Public login context is presentation metadata, never an opaque fetch input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UsageLoginFailure {
    /// Agent associated with this attempted login, when known.
    pub harness: Option<HarnessRef>,
    /// Whole source-owned label; never a path or credential.
    pub label: LimitName,
    /// Sanitized attempt result.
    pub failure: UsageAttemptFailure,
}

/// A retained success is immutable across a later refresh or failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SuccessfulUsageReport {
    /// Actual successful completion time, not the latest attempt time.
    pub at: DateTime<Utc>,
    /// Values from exactly this success.
    pub report: AvailableUsageReport,
}

/// The attempt that a refresh is replacing, separate from an older successful snapshot.
/// An omitted origin never promotes a retained failure to a current observation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum UsageRefreshOrigin {
    /// The immediately preceding attempt succeeded and remains visible while pending.
    Available,
    /// Failed, unavailable or unknown; success is retained only for the account card.
    #[default]
    Unavailable,
}

/// Every query state is explicit; a failure never promotes old values to current.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SourceReportState {
    /// A cache-only read found no previous attempt.
    NotRequested,
    /// One admitted fetch is running.
    Refreshing {
        /// Availability of the attempt being replaced, never inferred from snapshot presence.
        #[serde(default)]
        origin: UsageRefreshOrigin,
        /// Actual start when the source retained it; absent is never the query time.
        started_at: Option<DateTime<Utc>>,
        /// Previous success only for the same validated login binding.
        retained: Option<SuccessfulUsageReport>,
    },
    /// Latest attempt succeeded.
    Available {
        /// Successful values and their own completion time.
        observed: SuccessfulUsageReport,
    },
    /// Latest attempt established an unavailable state.
    Unavailable {
        /// Actual attempt completion when retained by the source; never the query time.
        attempted_at: Option<DateTime<Utc>>,
        /// Sanitized current reason.
        problem: UsageProblem,
        /// Actual provider cooldown when supplied; never a guessed retry delay.
        retry_at: Option<DateTime<Utc>>,
        /// Same-binding success only.
        retained: Option<SuccessfulUsageReport>,
        /// Independent safe errors when every login failed.
        logins: ReportLoginFailures,
    },
    /// Latest attempt failed to fetch a report.
    Failed {
        /// Actual attempt completion when retained by the source; unknown stays absent.
        attempted_at: Option<DateTime<Utc>>,
        /// Sanitized current error category.
        reason: UsageFetchFailure,
        /// Actual provider cooldown when supplied; never a guessed retry delay.
        retry_at: Option<DateTime<Utc>>,
        /// Same-binding success only.
        retained: Option<SuccessfulUsageReport>,
        /// Independent safe errors when every login failed.
        logins: ReportLoginFailures,
    },
}

/// One stable source/account identity, independent of changing labels and logins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SourceReportEntry {
    /// Opaque identity resolved through source discovery, not string matching.
    pub(super) id: UsageReportId,
    /// Explicit source identity; clients never split the opaque report handle.
    source: UsageSourceId,
    /// Explicit account identity scoped to that source, independent of its display label.
    account: UsageAccountKey,
    /// Whole source label.
    pub source_label: LimitName,
    /// Official bundled source mark; absent uses the shared plugin/gauge fallback.
    pub builtin: Option<BuiltinUsageSource>,
    /// Exact configured saved-login store for credential-use disclosure, if applicable.
    pub saved_login: Option<crate::limits::LimitLoginStore>,
    /// Optional human account label, never used for lookup or cache identity.
    pub account_label: Option<LimitName>,
    /// Current attempt and original successful observations.
    pub state: SourceReportState,
}

impl SourceReportEntry {
    /// Construct matching report/source/account identities atomically.
    #[must_use]
    pub fn new(
        id: UsageReportId,
        source_label: LimitName,
        builtin: Option<BuiltinUsageSource>,
        saved_login: Option<crate::limits::LimitLoginStore>,
        account_label: Option<LimitName>,
        state: SourceReportState,
    ) -> Self {
        Self {
            source: id.source().clone(),
            account: id.account().clone(),
            id,
            source_label,
            builtin,
            saved_login,
            account_label,
            state,
        }
    }
    /// Opaque handle for exact inspection; never parsed by a consumer for grouping.
    #[must_use]
    pub const fn id(&self) -> &UsageReportId {
        &self.id
    }
    /// Stable source identity for pins and grouping.
    #[must_use]
    pub const fn source(&self) -> &UsageSourceId {
        &self.source
    }
    /// Stable source-owned account identity, never its presentation label.
    #[must_use]
    pub const fn account(&self) -> &UsageAccountKey {
        &self.account
    }

    /// Every direct, retained and pending success receives the same scope projection.
    #[must_use]
    pub fn for_visibility(mut self, visibility: crate::usage::CostVisibility) -> Self {
        self.redact(visibility);
        self
    }
    pub(super) fn redact(&mut self, visibility: crate::usage::CostVisibility) {
        if visibility == crate::usage::CostVisibility::Permitted {
            return;
        }
        let observed = match &mut self.state {
            SourceReportState::NotRequested => None,
            SourceReportState::Available { observed } => Some(observed),
            SourceReportState::Refreshing { retained, .. }
            | SourceReportState::Unavailable { retained, .. }
            | SourceReportState::Failed { retained, .. } => retained.as_mut(),
        };
        if let Some(observed) = observed {
            observed.report.balances.redact_amounts();
            observed.report.details.read_only();
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryFields {
    id: UsageReportId,
    source: UsageSourceId,
    account: UsageAccountKey,
    source_label: LimitName,
    builtin: Option<BuiltinUsageSource>,
    saved_login: Option<crate::limits::LimitLoginStore>,
    account_label: Option<LimitName>,
    state: SourceReportState,
}
impl<'de> Deserialize<'de> for SourceReportEntry {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        let entry = EntryFields::deserialize(decoder)?;
        if entry.id.source() != &entry.source || entry.id.account() != &entry.account {
            return Err(serde::de::Error::custom(
                "usage report/source/account identities do not match",
            ));
        }
        Ok(Self::new(
            entry.id,
            entry.source_label,
            entry.builtin,
            entry.saved_login,
            entry.account_label,
            entry.state,
        ))
    }
}
