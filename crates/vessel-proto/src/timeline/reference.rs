//! Read-only provenance for tool references returned through fleet envelopes.
//! Only the negotiated node connection can classify legacy ingress. This value
//! can reduce a reference to display-only; no value asserts canonical authority.
use serde::{Deserialize, Serialize};

/// Origin of every tool reference within one forwarded reply.
/// Absence is canonical only on a peer supporting `TIMELINE_PARITY`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TimelineReferenceOrigin {
    /// Already received in legacy form; never project again or resolve by spelling.
    /// Unknown future origins conservatively take the same downgrade-only state.
    #[serde(other)]
    LegacyProjected,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventPage, EventSeq, Forwarded, Reply};
    #[test]
    fn unknown_node_reply_origin_can_only_downgrade_to_legacy() {
        let reply = Forwarded {
            reference_origin: Some(TimelineReferenceOrigin::LegacyProjected),
            reply: Box::new(Reply::ReadEvents(EventPage {
                records: vec![],
                next: EventSeq::ZERO,
                at_end: true,
            })),
        };
        let mut encoded = serde_json::to_value(&reply).unwrap();
        encoded["reference_origin"] = serde_json::json!("future_origin");
        assert_eq!(serde_json::from_value::<Forwarded>(encoded).unwrap(), reply);
        let plain = Forwarded {
            reference_origin: None,
            ..reply
        };
        assert!(
            serde_json::to_value(plain)
                .unwrap()
                .get("reference_origin")
                .is_none()
        );
    }
}
