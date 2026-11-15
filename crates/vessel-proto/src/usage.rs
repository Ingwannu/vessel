//! Reported usage, separate from context occupancy (ADR 0042).

use crate::conversation::Cost;
use crate::{EventSeq, HarnessRef, ProfileName, SessionId, TurnUsage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Methods and replay provenance share one compatibility boundary: a peer
// must understand both before it can distinguish current from loaded usage.
pub(crate) const USAGE_PROTOCOL_VERSION: crate::ProtocolVersion = crate::ProtocolVersion::new(21);

/// A count of reports, bounded by the retained ledger.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct ReportCount(u32);
impl ReportCount {
    /// Wrap a count.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
    /// Number of reports.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
    /// Add a report; saturation preserves safety at the counter's bound.
    pub fn increment(&mut self) {
        self.0 = self.0.saturating_add(1);
    }
}

/// Rolling periods; a day is 24 hours, a week seven days.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum UsagePeriod {
    /// Past 24 hours.
    Day,
    /// Past seven days.
    Week,
    /// All retained samples of one session only.
    Session,
}

/// Which usage the caller asks for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UsageScope {
    /// All agents on this node.
    Machine,
    /// One agent, across all profiles or one recorded profile.
    Agent {
        /// Agent implementation.
        harness: HarnessRef,
        /// None includes every profile. Some selects that recorded name.
        profile: Option<ProfileName>,
    },
    /// One conversation.
    Session {
        /// Session identity.
        session: SessionId,
    },
}

/// `usage.query` (protocol 21). Totals cover the selected range; `after`
/// pages only the individual session samples, never the total.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryUsage {
    /// What is counted.
    pub scope: UsageScope,
    /// Selected rolling interval.
    pub period: UsagePeriod,
    /// Exclusive sequence cursor; only valid for a session scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<EventSeq>,
}

/// Sums of values actually reported, never estimates. Optional cache and
/// reasoning counts sum only the turns reporting those fields.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UsageTotals {
    /// None when no turn supplied token counts; reported zero stays Some.
    pub tokens: Option<TurnUsage>,
    /// Whether a sum exceeded the wire counter range.
    pub token_accounting: TokenAccounting,
    /// Separate totals per unit; empty when absent or not permitted.
    pub costs: Vec<Cost>,
    /// Turns that supplied tokens.
    pub reported: ReportCount,
    /// Ended turns without a token report.
    pub unreported: ReportCount,
    /// Cost samples omitted for unknown timing or numerical precision.
    /// An unrepresentable unit sum counts every contributing cost sample.
    pub unaccounted_costs: ReportCount,
    /// How many turns supplied each optional token field. Compare each
    /// count with reported + unreported before calling it a complete sum.
    pub optional_reports: OptionalUsageReports,
}

/// One agent/profile's totals on the queried machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AgentUsage {
    /// Which agent.
    pub harness: HarnessRef,
    /// Profile at the time of the sample, not its current assignment.
    pub profile: Option<ProfileName>,
    /// Its reported usage.
    pub totals: UsageTotals,
}

/// One committed turn or cost increment. The event sequence is its stable
/// identity. Cost updates are not assumed to describe a whole turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UsageSample {
    /// Event that supplied the report.
    pub seq: EventSeq,
    /// When the node recorded it.
    pub at: DateTime<Utc>,
    /// The mutually exclusive report kind.
    pub kind: UsageSampleKind,
}

/// A bounded, retained usage report. No claim of billing completeness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UsageReport {
    /// Query time.
    pub at: DateTime<Utc>,
    /// When this node first enabled accounting; no historical backfill.
    pub accounting_started: DateTime<Utc>,
    /// Earliest time retained by the age policy.
    pub retained_from: DateTime<Utc>,
    /// The node pruned samples in this interval, or omitted agent groups.
    /// Coverage of the selected scope may be partial; totals still count
    /// every retained selected sample.
    pub incomplete: bool,
    /// Whether cost was withheld by the destination scope check.
    pub cost_visibility: CostVisibility,
    /// For a session query, its latest cumulative agent reports. These
    /// are observations, not money assigned to the selected time period.
    pub current_costs: Vec<Cost>,
    /// Selected totals.
    pub totals: UsageTotals,
    /// At most 256 groups, in stable agent/profile order.
    pub agents: Vec<AgentUsage>,
    /// At most 100 session samples, oldest first after the cursor.
    pub samples: Vec<UsageSample>,
    /// Cursor for the next session page, only when more remain.
    pub next: Option<EventSeq>,
}

/// A turn end and a running cost update are different sources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UsageSampleKind {
    /// One ended turn, possibly without counts.
    Turn {
        /// Exactly what the agent reported.
        usage: Option<TurnUsage>,
    },
    /// An agent reported cost outside exact arithmetic or wire precision.
    CostUnavailable,
    /// The first observation of an existing/unknown/reset counter. It
    /// is shown as a cumulative report, never charged to today.
    CostBaseline {
        /// What the agent said, without inventing its timing.
        cost: Cost,
    },
    /// A nonnegative increment from the agent's running cost.
    Cost {
        /// Beginning of the observation interval. Queries that cut this
        /// interval do not assign its whole cost to the shorter period.
        since: DateTime<Utc>,
        /// Never mixed with another unit.
        cost: Cost,
    },
}

/// Cost absence and scope redaction are different user-visible states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CostVisibility {
    /// The caller may see reported cost; an empty list means not reported.
    Permitted,
    /// Cost is withheld by this caller's scopes.
    Withheld,
}

/// Saturation is explicit; a capped counter is never presented as exact.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TokenAccounting {
    /// Sums fit the wire counters.
    #[default]
    Reported,
    /// At least one counter exceeded `u64::MAX`; displayed totals are partial.
    Overflow,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MIN_PROTOCOL_VERSION, PROTOCOL_VERSION, ProtocolVersion, Request as _, Scope};
    #[test]
    fn usage_is_additive_and_old_connections_refuse_the_new_call() {
        let request = QueryUsage {
            scope: UsageScope::Machine,
            period: UsagePeriod::Day,
            after: None,
        };
        let call = request.clone().into_call();
        assert!(call.since() <= Some(PROTOCOL_VERSION));
        assert_eq!(call.method().scope(), Some(Scope::Read));
        assert_eq!(MIN_PROTOCOL_VERSION, ProtocolVersion::new(2));
        assert_eq!(
            serde_json::from_str::<QueryUsage>(&serde_json::to_string(&request).unwrap()).unwrap(),
            request
        );
        assert!(
            serde_json::from_str::<QueryUsage>(
                r#"{"scope":{"kind":"machine"},"period":"unknown"}"#
            )
            .is_err()
        );
    }

    fn envelopes(call: crate::Call) -> [crate::Call; 4] {
        let node = crate::NodeId::random();
        let origin = crate::Origin {
            node,
            by: crate::OriginBy::Owner,
            scopes: vec![Scope::Read],
        };
        [
            call.clone(),
            crate::Forward {
                node,
                call: Box::new(call.clone()),
            }
            .into_call(),
            crate::RelayCall {
                node,
                call: Box::new(call.clone()),
                origin: origin.clone(),
            }
            .into_call(),
            crate::Deliver {
                call: Box::new(call),
                origin,
            }
            .into_call(),
        ]
    }

    #[test]
    fn usage_envelopes_require_the_inner_method_and_fields_while_old_calls_stay_supported() {
        let query = QueryUsage {
            scope: UsageScope::Machine,
            period: UsagePeriod::Day,
            after: None,
        };
        assert_eq!(
            envelopes(query.into_call()).map(|call| call.since()),
            [QueryUsage::METHOD.since(); 4]
        );
        let six = ProtocolVersion::new(6);
        assert_eq!(
            envelopes(crate::GetNode.into_call()).map(|call| call.since()),
            [
                crate::GetNode::METHOD.since(),
                Some(six),
                Some(six),
                Some(six)
            ]
        );
        let fields = crate::settings::SetNodeSettings {
            drivers: Some(crate::settings::SetAgentDrivers::default()),
            ..Default::default()
        };
        assert_eq!(
            envelopes(fields.into_call()).map(|call| call.since()),
            [Some(ProtocolVersion::new(18)); 4]
        );
    }
}

/// Coverage of optional fields, independent of whether a turn supplied
/// its mandatory input/output counters. Zero remains a reported value.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OptionalUsageReports {
    /// Turns reporting reasoning tokens.
    pub thought: ReportCount,
    /// Turns reporting cache reads.
    pub cached_read: ReportCount,
    /// Turns reporting cache writes.
    pub cached_write: ReportCount,
}
