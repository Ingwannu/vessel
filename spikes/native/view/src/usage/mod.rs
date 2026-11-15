//! Usage request lifetimes and presentation rules; the node owns accounting.
//!
//! A scope change must never relabel another account's last successful data.
//! The reader bounds admission to one active read and the newest queued choice.

mod accounts;
pub use accounts::{Account, configured_accounts};
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
pub use inventory::{AccountReport, Inventory, account_binding};

mod session;
pub use session::{SessionSelection, binding_epoch};

mod quota;
pub use quota::{quota_facts, reset_label, window_label};

mod summary;
pub use summary::{PinnedWindow, SummaryAccount, SummaryWindow, default_pins, pinned_windows};

mod summary_layout;
pub use summary_layout::{
    MIN_SUMMARY_METER_WIDTH, SUMMARY_METER_GAP, SummaryLayout, SummaryMeasurements, SummaryWidth,
    SummaryWindowCount, summary_layout,
};
