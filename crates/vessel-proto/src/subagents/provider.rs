//! Provider-owned tasks have no managed session id or lifecycle operation rights.

use super::{ProviderChildId, ProviderText, bounds};
use crate::{EventSeq, SessionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// An actual provider lifecycle report; absent status remains unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ChildStatus {
    /// The provider observed ongoing child work.
    Running,
    /// The child completed successfully.
    Completed,
    /// The provider reported failure.
    Failed,
    /// The provider reported cancellation.
    Canceled,
}

/// The provider identity is scoped by the managed parent in the request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ChildDescriptor {
    /// Actual provider task/tool/thread id, never a fabricated managed session.
    pub id: ProviderChildId,
    /// Actual provider nesting, when reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<ProviderChildId>,
    /// Provider label; the native track prefers description when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<ProviderText>,
    /// Task description supplied by the provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<ProviderText>,
    /// Supplemental provider label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<ProviderText>,
    /// No report is unknown, never implicitly completed or running.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ChildStatus>,
    /// Actual provider timestamp, unknown when not reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Actual provider timestamp, unknown when not reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Honest reasons a projection cannot show the full provider history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ChildGap {
    /// The durable parent log no longer retains its beginning.
    Retention,
    /// The bounded scan reached its record/byte/deadline budget.
    ScanBudget,
    /// The provider-side translator reached its retained state budget.
    IngestionBudget,
    /// A provider nesting chain was too deep or cyclic.
    Nesting,
}

/// Provider support and missing history are distinct from an empty successful list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ChildHistory {
    /// All relevant durable records were available and read.
    Complete,
    /// Some history is unavailable; current observations are still useful.
    Partial {
        /// Cause, with no raw provider error text.
        reason: ChildGap,
        /// First parent sequence actually scanned.
        from: EventSeq,
    },
    /// This agent/driver does not report provider children.
    Unsupported,
    /// The named provider child has no retained descriptor/history.
    Missing,
}

/// Planned provider list input; permissions remain the existing session read policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListProviderChildren {
    /// Managed parent on the receiving node.
    pub session: SessionId,
}

/// Parent sequences are the only cursor; a child has no second epoch/log number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "direction", content = "seq", rename_all = "snake_case")]
pub enum ChildCursor {
    /// Records preceding this parent sequence, exclusive.
    Before(EventSeq),
    /// Records at/after the parent's continuation sequence.
    After(EventSeq),
}

/// A nonzero bounded page size, validated at the wire boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(try_from = "u32", into = "u32")]
pub struct ChildLimit(u32);

/// A page size cannot request an unbounded scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("child timeline limit must be between one and its maximum")]
pub struct InvalidChildLimit;

impl TryFrom<u32> for ChildLimit {
    type Error = InvalidChildLimit;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value == 0 || value > bounds::MAX_TIMELINE_ITEMS {
            return Err(InvalidChildLimit);
        }
        Ok(Self(value))
    }
}
impl From<ChildLimit> for u32 {
    fn from(value: ChildLimit) -> Self {
        value.0
    }
}
impl Default for ChildLimit {
    fn default() -> Self {
        Self(bounds::MAX_TIMELINE_ITEMS)
    }
}

/// Planned provider child timeline input; absent cursor means the retained tail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReadProviderChild {
    /// Managed parent on the receiving node.
    pub session: SessionId,
    /// Provider task identity within that parent.
    pub child: ProviderChildId,
    /// Stable parent-log continuation; no fabricated child epoch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<ChildCursor>,
    /// Defaults to the maximum bounded page size.
    #[serde(default)]
    pub limit: ChildLimit,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_and_oversized_limits_cannot_be_decoded_or_constructed() {
        assert!(ChildLimit::try_from(0).is_err());
        assert!(ChildLimit::try_from(bounds::MAX_TIMELINE_ITEMS.saturating_add(1)).is_err());
        assert!(serde_json::from_str::<ChildLimit>("0").is_err());
        assert_eq!(u32::from(ChildLimit::default()), bounds::MAX_TIMELINE_ITEMS);
        assert_eq!(u32::from(ChildLimit::try_from(1).unwrap()), 1);
    }
    #[test]
    fn provider_ids_and_unknown_fields_do_not_turn_into_managed_rights_or_status() {
        let descriptor: ChildDescriptor = serde_json::from_str(r#"{"id":"toolu-task-1"}"#).unwrap();
        assert_eq!(descriptor.status, None);
        assert_eq!(descriptor.created_at, None);
        assert_eq!(descriptor.updated_at, None);
        assert_eq!(descriptor.parent, None);
        let text = serde_json::to_string(&descriptor).unwrap();
        assert_eq!(text, r#"{"id":"toolu-task-1"}"#);
        assert!(serde_json::from_str::<ChildDescriptor>(r#"{"id":"bad\nchild"}"#).is_err());
    }
    #[test]
    fn parent_sequence_cursor_and_missing_support_are_distinct_wire_values() {
        let cursor = ChildCursor::After(EventSeq::new(42));
        assert_eq!(
            serde_json::from_str::<ChildCursor>(&serde_json::to_string(&cursor).unwrap()).unwrap(),
            cursor
        );
        for history in [
            ChildHistory::Complete,
            ChildHistory::Unsupported,
            ChildHistory::Missing,
            ChildHistory::Partial {
                reason: ChildGap::Retention,
                from: EventSeq::new(9),
            },
        ] {
            assert_eq!(
                serde_json::from_str::<ChildHistory>(&serde_json::to_string(&history).unwrap())
                    .unwrap(),
                history
            );
        }
    }
}
