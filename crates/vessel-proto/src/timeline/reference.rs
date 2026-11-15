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

/// One outgoing traversal carries provenance separately from provider-controlled ids.
#[derive(Clone, Copy)]
pub(crate) struct ReferenceContext {
    pub(crate) origin: Option<TimelineReferenceOrigin>,
    pub(crate) assigned: Option<crate::ProtocolVersion>,
}

impl ReferenceContext {
    pub(crate) const CANONICAL: Self = Self {
        origin: None,
        assigned: super::TIMELINE_PARITY,
    };

    pub(crate) fn supports(self, protocol: crate::ProtocolVersion) -> bool {
        self.assigned.is_some_and(|since| protocol >= since)
    }

    pub(crate) fn within(self, origin: Option<TimelineReferenceOrigin>) -> Self {
        Self {
            origin: self.origin.or(origin),
            ..self
        }
    }

    pub(crate) fn project(
        self,
        id: &crate::ToolCallId,
        scope: Option<&super::ToolCallScope>,
    ) -> crate::ToolCallId {
        match self.origin {
            Some(TimelineReferenceOrigin::LegacyProjected) => id.clone(),
            None => super::compat::identity::legacy_id(id, scope),
        }
    }
}

impl crate::Forwarded {
    /// Refuse scope-dependent mutations on display-only or staging fleet data.
    /// Permission answers use their own `PermissionId` capability and do not use this resolver.
    /// # Errors
    /// Legacy or unassigned references cannot identify a canonical scoped target.
    pub fn check_scoped_references(&self) -> Result<(), crate::ErrorCode> {
        if super::TIMELINE_PARITY.is_none() || self.reference_origin.is_some() {
            return Err(crate::ErrorCode::Unsupported);
        }
        Ok(())
    }

    /// Bind references received from an actual negotiated node connection.
    /// An older connection can only reduce them to display-only legacy data.
    pub fn received_from(&mut self, protocol: crate::ProtocolVersion) {
        self.received_with(protocol, super::TIMELINE_PARITY);
    }

    pub(crate) fn received_with(
        &mut self,
        protocol: crate::ProtocolVersion,
        assigned: Option<crate::ProtocolVersion>,
    ) {
        if assigned.is_none_or(|since| protocol < since) {
            self.reference_origin = Some(TimelineReferenceOrigin::LegacyProjected);
        }
    }
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
