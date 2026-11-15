//! Source-provided values remain independent of ledger accounting and UI defaults.
use super::{ReportBalances, ReportDetails, ReportWindows};
use crate::{
    conversation::FiniteNumber,
    limits::{LimitKey, LimitName, LimitPercent},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Tone is a source fact or a presentation fallback, never a quota percentage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum UsageTone {
    /// Ordinary neutral presentation.
    Default,
    /// Source explicitly reports healthy usage.
    Ok,
    /// Source warns of an approaching limit.
    Warning,
    /// Source reports danger or exhaustion.
    Danger,
}

/// Known bundled sources can use the client's existing official marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum BuiltinUsageSource {
    /// Claude's subscription usage.
    Claude,
    /// `ChatGPT` subscription usage, including supported alternate login stores.
    Codex,
    /// GitHub Copilot plan metadata.
    Copilot,
    /// Cursor plan and third-party allowances.
    Cursor,
    /// Grok CLI credits and period.
    Grok,
    /// Kimi coding allowances.
    Kimi,
    /// `MiniMax` coding plans.
    MiniMax,
    /// `OpenCode` Go allowances.
    OpenCodeGo,
    /// Z.ai subscription metadata.
    Zai,
}

/// An omitted short label follows the full label; Hidden explicitly suppresses it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", content = "text", rename_all = "snake_case")]
pub enum UsageShortLabel {
    /// An unknown period must not acquire an invented short duration.
    Hidden,
    /// Source-supplied short text.
    Text(LimitName),
}

/// Default summary choices are explicit source metadata.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum UsageSummary {
    /// Fall back to the first percentage window when none are marked.
    #[default]
    Unspecified,
    /// Include this window in the default sidebar selection.
    Included,
}

/// Independent allowances are never summed into a single percentage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UsageWindow {
    /// Stable identity within the source's report.
    pub id: LimitKey,
    /// Whole source-provided label.
    pub label: LimitName,
    /// Explicit short name or suppression.
    pub short_label: Option<UsageShortLabel>,
    /// Source's default sidebar selection.
    #[serde(default)]
    pub summary: UsageSummary,
    /// Reported used share; absence is not zero.
    pub used: Option<LimitPercent>,
    /// Reported remaining share; it takes priority in remaining mode.
    pub remaining: Option<LimitPercent>,
    /// Source-provided reset instant, never inferred from window position.
    pub resets_at: Option<DateTime<Utc>>,
    /// Optional source forecast, distinct from a reset.
    pub runs_out_at: Option<DateTime<Utc>>,
    /// Optional forecast shortfall; no client derives this from usage.
    pub shortfall: Option<LimitPercent>,
    /// Source tone overrides the client's threshold fallback.
    pub tone: Option<UsageTone>,
}

/// Balances retain the vendor's unit; only USD is a currency convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum UsageBalanceUnit {
    /// Reported US dollars, never converted by Vessel.
    Usd,
    /// Vendor-defined credits.
    Credits,
    /// Count of requests.
    Requests,
    /// Token allowance, separate from reported ledger tokens.
    Tokens,
}

/// Optional balance fields remain independent; missing values are not estimated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UsageBalance {
    /// Stable balance identity within this report.
    pub id: LimitKey,
    /// Whole source label.
    pub label: LimitName,
    /// Reported amount used.
    pub used: Option<FiniteNumber>,
    /// Reported balance remaining.
    pub remaining: Option<FiniteNumber>,
    /// Reported allowance; zero is distinct from missing.
    pub limit: Option<FiniteNumber>,
    /// Source unit, not a price catalogue.
    pub unit: UsageBalanceUnit,
    /// Optional source reset.
    pub resets_at: Option<DateTime<Utc>>,
    /// Source tone when provided.
    pub tone: Option<UsageTone>,
}

/// Missing classification is private by default, avoiding accidental money disclosure.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum UsageDetailAccess {
    /// Safe non-monetary metadata available with Read.
    Read,
    /// Requires Operate, just as reported cost and credit balances do.
    #[default]
    Operate,
}

/// A bounded metadata row carries no opaque executable or provider error body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UsageDetail {
    /// Stable row identity.
    pub id: LimitKey,
    /// Whole source label.
    pub label: LimitName,
    /// Source metadata, bounded and single-line like the label.
    pub value: LimitName,
    /// Safe visibility classification; omission requires Operate.
    #[serde(default)]
    pub access: UsageDetailAccess,
    /// Optional semantic source tone.
    pub tone: Option<UsageTone>,
}

/// Whether malformed or bounded provider entries made an observation partial.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum UsageCompleteness {
    /// No provider data was omitted by this source.
    #[default]
    Complete,
    /// The source explicitly reports omitted or malformed provider entries.
    Partial,
}

/// Values belong only to an explicitly successful observation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AvailableUsageReport {
    /// Partial parsing is disclosed separately from successful transport.
    #[serde(default)]
    pub completeness: UsageCompleteness,
    /// Recognized subscription name, never an estimated price.
    pub plan: Option<LimitName>,
    /// Independent bounded quota windows.
    pub windows: ReportWindows,
    /// Independent bounded balances.
    pub balances: ReportBalances,
    /// Non-executable bounded metadata rows.
    pub details: ReportDetails,
}
