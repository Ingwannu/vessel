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

/// Assignment waits for native/page/CLI report consumers and the plugin source
/// value adapter; a cached read must never trigger credential or vendor I/O.
pub const ACCOUNT_REPORTS_PROTOCOL: Option<crate::ProtocolVersion> = None;

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
