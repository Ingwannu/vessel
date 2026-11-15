//! Usage request lifetimes and presentation rules; the node owns accounting.
//!
//! A scope change must never relabel another account's last successful data.
//! The reader bounds admission to one active read and the newest queued choice.

mod accounts;
pub use accounts::{
    Account, InvalidAccountInventory, MAX_CONFIGURED_USAGE_ACCOUNTS, configured_accounts,
};
mod display;
pub use display::{DisplayAs, DisplayPercent, timestamp};
mod range;
pub use range::{LedgerSelection, Range};
mod hosts;
pub use hosts::{HostAvailability, HostChoice, HostSupport, UsageHost};
mod pins;
pub use pins::{Pin, PinSelection};
mod tone;
pub use tone::{Tone, window_tone};
mod reader;
mod tokens;
pub use reader::{Phase, ReadRequest, Reader, RequestFailure};
pub use tokens::{TokenField, token_count};

mod report;
pub use report::{ReportFact, coverage_facts, partial_report, total_facts};
mod limits;
pub use limits::{LimitStatus, LimitView, limit_failure, limit_view};
mod inventory;
pub use inventory::{
    AccountReport, Inventory, InventoryRefresh, InventorySurface, account_binding,
};

mod session;
pub use session::{SessionSelection, binding_epoch};

mod quota;
pub use quota::{quota_facts, reset_label, window_label};

mod source_summary;
mod source_values;
mod sources;
pub use source_values::{
    SourceWindowTiming, balance_percent, source_risk_label, source_window_timing,
};
mod summary;
pub use source_summary::{builtin_source_mark, saved_source, source_mark, summary_candidates};
pub use sources::{
    SourceView, balance_facts, source_availability, source_disclosure, source_failure,
    source_problem, source_view, source_window_tone,
};
pub use summary::{
    PinnedWindow, SummaryAccount, SummaryGroup, SummaryWindow, default_pins, pinned_windows,
};

mod summary_layout;
pub use summary_layout::{
    MIN_SUMMARY_METER_WIDTH, SUMMARY_METER_GAP, SummaryLayout, SummaryMeasurements, SummaryWidth,
    SummaryWindowCount, summary_layout,
};
#[cfg(test)]
mod wording_tests;
