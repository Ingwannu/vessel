//! Source/account reports for cards, pins and CLI; separate from token accounting.
//! Pending method rows refuse transport admission until the integrator assigns
//! the feature and its real source implementations are ready (ADR 0042).
mod identity;
mod saved;
pub use identity::{
    InvalidUsageIdentity, MAX_USAGE_ID_COMPONENT_BYTES, UsageAccountKey, UsageReportId,
    UsageSourceId,
};
pub use saved::SavedUsageSource;

/// Protocol 26 (2026-11-16): `usage.reports` and `usage.inspect` (Read, unary,
/// fleet-forwardable). Assigned after the provider client resolves names only
/// through the node's tracked DNS scope (ADR 0104); the coordinator ruled that
/// this activation lands before its native/page/CLI consumers, which fall back
/// to `usage.limits` (23) on older or unassigned peers. A cached read never
/// triggers credential or vendor I/O. Kept separate from
/// [`USAGE_SOURCE_CONSENT_PROTOCOL`] so either can move on its own later.
pub const ACCOUNT_REPORTS_PROTOCOL: Option<crate::ProtocolVersion> =
    Some(crate::ProtocolVersion::new(26));

mod budget;
pub use budget::{EncodedReportBytes, MAX_USAGE_RESPONSE_BYTES, UsageReportBudgetError};
mod bounded;
pub use bounded::{
    InvalidUsageCollection, MAX_USAGE_BALANCES, MAX_USAGE_DETAILS, MAX_USAGE_LOGIN_FAILURES,
    MAX_USAGE_REPORTS, MAX_USAGE_SOURCE_FAILURES, MAX_USAGE_WINDOWS, ReportBalances, ReportDetails,
    ReportEntries, ReportLoginFailures, ReportSourceDescriptors, ReportSourceFailures,
    ReportSourceGrants, ReportWindows, UsageCollection,
};
mod values;
pub use values::{
    AvailableUsageReport, BuiltinUsageSource, UsageBalance, UsageBalanceUnit, UsageCompleteness,
    UsageDetail, UsageDetailAccess, UsageShortLabel, UsageSummary, UsageTone, UsageWindow,
};
mod state;
pub use state::{
    NoQuotaReason, SourceReportEntry, SourceReportState, SuccessfulUsageReport,
    UsageAttemptFailure, UsageFetchFailure, UsageLoginFailure, UsageProblem, UsageRefreshOrigin,
};
mod disclosure;
mod requests;
pub use disclosure::{
    SetUsageSourceLookup, USAGE_SOURCE_CONSENT_PROTOCOL, UsageCredentialStore,
    UsageDisclosureVersion, UsageRecipient, UsageSourceAvailability, UsageSourceConsent,
    UsageSourceDescriptor, UsageSourceDisclosure, UsageSourceGrant,
};
pub use requests::{
    InspectUsageReport, ListUsageReports, UsageReportLookup, UsageReportScope, UsageReports,
    UsageSourceFailure,
};

#[cfg(test)]
mod tests;
