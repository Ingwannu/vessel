//! Latest saved-sign-in quota facts; never credentials or billing estimates.
use crate::conversation::FiniteNumber;
use crate::usage::CostVisibility;
use crate::{DurationMs, HarnessRef, ProfileName, SessionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
mod text;
pub use text::{InvalidLimitText, LimitKey, LimitName};

// `usage.limits` and `usage.lookup` arrived after `usage.query` (21) and the
// inbox (22) on the node line, so they need a protocol of their own: the
// track had one boundary for all three (provisionally 19), but a peer at 21
// or 22 knows usage without these calls.
pub(crate) const LIMITS_PROTOCOL_VERSION: crate::ProtocolVersion = crate::ProtocolVersion::new(23);

/// Permission for this node to use configured agents' saved sign-ins locally.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ExternalLookup {
    /// Read no saved sign-in and forget external snapshots.
    #[default]
    Off,
    /// Permit bounded requests to the fixed vendor origins on this machine.
    On,
}
/// Cached read or explicitly requested vendor refresh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum LimitRefresh {
    /// No credential or vendor I/O.
    Cached,
    /// Requires operate permission and obeys cooldown/admission.
    Refresh,
}
/// One configured login; an omitted profile means the configured default,
/// unlike usage.query's all-history profile selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LimitAccount {
    /// Which agent runs here.
    pub harness: HarnessRef,
    /// Selected configured profile; None is the default.
    pub profile: Option<ProfileName>,
}
/// A saved account or the saved login of a session's selected profile.
/// This does not prove what credential an already-running process remembers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LimitsScope {
    /// One binding; profiles are enumerated by the existing profile.list.
    Account {
        /// Selected configured binding.
        account: LimitAccount,
    },
    /// Agent principals can request only their own local session.
    Session {
        /// Session whose selected profile is inspected.
        session: SessionId,
    },
}
/// usage.limits: read cached facts; operate additionally permits refresh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetUsageLimits {
    /// Selected saved login, never a path or endpoint.
    pub scope: LimitsScope,
    /// Whether to request a bounded refresh.
    pub refresh: LimitRefresh,
}
/// usage.lookup: administrators alone change the machine credential-use policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SetUsageLookup {
    /// Explicit policy, persisted before acknowledgement.
    pub policy: ExternalLookup,
}
/// Saved login source, for a precise credential-use disclosure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum LimitLoginStore {
    /// Claude Code's configured credential file, or its macOS Keychain item.
    ClaudeCode,
    /// Codex's configured auth.json.
    Codex,
    /// `OpenCode`'s saved `OpenAI` OAuth login in its data directory.
    OpenCode,
}
/// Finite vendor-reported percentage; drawing clamps belong to consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct LimitPercent(FiniteNumber);
impl LimitPercent {
    /// Wrap an already validated finite report without inferring a range.
    #[must_use]
    pub const fn reported(number: FiniteNumber) -> Self {
        Self(number)
    }
    /// Raw reported percentage, possibly outside the drawing's 0..100 range.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0.get()
    }
}
/// Only a reported length identifies a period; slot position never does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LimitPeriod {
    /// Explicit five-hour allowance.
    FiveHour,
    /// Explicit seven-day allowance.
    Weekly,
    /// Another reported duration.
    Other {
        /// Vendor-supplied duration.
        duration: DurationMs,
    },
    /// No length was reported.
    Unknown,
}
/// Position identifies an unknown window without inventing its duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum LimitSlot {
    /// A separately named allowance.
    Only,
    /// Provider's primary slot.
    Primary,
    /// Provider's secondary slot.
    Secondary,
}
/// Independent operation a quota applies to; scoped windows are not summed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LimitScope {
    /// Ordinary account use.
    General,
    /// One model family.
    Model {
        /// Verbatim vendor identity when supplied.
        id: Option<LimitKey>,
        /// Bounded presentation name.
        name: LimitName,
    },
    /// One product surface.
    Surface {
        /// Verbatim vendor identity when supplied.
        id: Option<LimitKey>,
        /// Bounded presentation name.
        name: LimitName,
    },
    /// A named metered feature.
    Feature {
        /// Verbatim feature identity.
        id: LimitKey,
        /// Bounded presentation name.
        name: LimitName,
    },
    /// Code review, separate from ordinary coding.
    CodeReview,
}
/// One independent reported allowance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LimitWindow {
    /// Stable identity in this response.
    pub id: LimitKey,
    /// Only vendor-reported duration is classified.
    pub period: LimitPeriod,
    /// Distinguishes slots when length is missing.
    pub slot: LimitSlot,
    /// Which work it meters.
    pub scope: LimitScope,
    /// None is unknown, distinct from a reported zero.
    pub used: Option<LimitPercent>,
    /// Only the vendor's reset; no inferred reset from a slot.
    pub resets_at: Option<DateTime<Utc>>,
}
/// Whether the provider explicitly enabled additional paid use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ExtraUsage {
    /// Additional paid use enabled.
    Enabled,
    /// Additional paid use disabled.
    Disabled,
}
/// Reported credits, without assigning a currency or estimating a price.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LimitCredits {
    /// Reported balance, withheld without operate permission.
    pub balance: Option<FiniteNumber>,
    /// Provider's reported eligibility, never inferred from a balance.
    pub has_credits: Option<bool>,
    /// Whether the provider reported unlimited credits.
    pub unlimited: Option<bool>,
}
/// Last successful sanitized response; at most 128 windows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LimitSnapshot {
    /// Observation time stays unchanged on a failed refresh.
    pub at: DateTime<Utc>,
    /// Recognized vendor subscription label.
    pub plan: Option<LimitName>,
    /// Independent allowances, never a combined percentage.
    pub windows: Vec<LimitWindow>,
    /// Optional provider credit facts.
    pub credits: Option<LimitCredits>,
    /// Optional paid-use switch reported by Claude.
    pub extra_usage: Option<ExtraUsage>,
    /// Optional explicit provider permission to continue.
    pub allowed: Option<bool>,
    /// Optional explicit provider exhaustion flag.
    pub limit_reached: Option<bool>,
    /// Some entries were malformed or exceeded the bounded representation.
    pub incomplete: bool,
}
/// Safe failure categories; raw credential/API errors never cross the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum LimitFailure {
    /// No saved OAuth login.
    NoLogin,
    /// The agent must refresh its expired sign-in.
    Expired,
    /// The vendor rejected the saved sign-in.
    Rejected,
    /// The fixed vendor origin could not be reached.
    Unreachable,
    /// Its response could not be represented safely.
    ChangedResponse,
    /// This agent/source has no supported limit lookup.
    Unsupported,
    /// The local sign-in could not be read safely.
    UnsafeLoginStore,
    /// This session uses another provider route.
    DifferentRoute,
    /// Its actual launch's saved-sign-in binding is no longer available.
    Unassociated,
    /// Vendor cooldown must elapse before another request.
    RateLimited,
    /// Current local checks must finish before another can be admitted.
    Busy,
}
/// Refresh state, separate from the successful observation's age.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AccountLimitState {
    /// External lookup is off; no snapshot retained.
    Off,
    /// Enabled but no refresh has been requested.
    NotRequested,
    /// One bounded request is in progress.
    Refreshing {
        /// Last safe response, if any.
        retained: Option<LimitSnapshot>,
    },
    /// Last refresh succeeded.
    Ready {
        /// Latest successful observation.
        snapshot: LimitSnapshot,
    },
    /// Last refresh failed, possibly retaining the same login's snapshot.
    Failed {
        /// Safe action category.
        reason: LimitFailure,
        /// Previous same-login response only.
        retained: Option<LimitSnapshot>,
        /// Actual cooldown when known.
        retry_at: Option<DateTime<Utc>>,
    },
}
/// Facts a session notice can draw; not proof why a turn stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LimitNotice {
    /// Reported full general windows, identified in the successful snapshot.
    pub reported_full: Vec<LimitKey>,
    /// Latest reset only when all selected full windows have future resets.
    pub latest_reported_reset: Option<DateTime<Utc>>,
}
/// One saved account's bounded facts and explicit credential-use policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UsageLimits {
    /// Administrator's machine-wide choice.
    pub policy: ExternalLookup,
    /// The selected configured binding.
    pub account: LimitAccount,
    /// Intended saved-login store for disclosure, never a credential path.
    pub source: Option<LimitLoginStore>,
    /// Latest safe state; never a claim about a running process's in-memory login.
    pub state: AccountLimitState,
    /// Visibility of monetary/credit balances.
    pub cost_visibility: CostVisibility,
    /// Optional factual full-window notice for a matching selected route.
    pub notice: Option<LimitNotice>,
}
