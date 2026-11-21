//! Compatibility and reply checks for a heartbeat's revision-bound cadence.
use vessel_proto::{
    ProtocolVersion,
    automation::{Cadence, CatchUp, EndReason, HEARTBEAT_CADENCE_PROTOCOL, ScheduleInfo, ScheduleState, UpdateScheduleCadence},
};

/// The feature has to be assigned and negotiated on the original peer.
#[must_use]
pub fn supported(protocol: Option<ProtocolVersion>) -> bool {
    admits(HEARTBEAT_CADENCE_PROTOCOL, protocol)
}

fn admits(required: Option<ProtocolVersion>, protocol: Option<ProtocolVersion>) -> bool {
    required.is_some_and(|required| protocol.is_some_and(|protocol| protocol >= required))
}

/// Unknown saved values remain readable until the client can preserve them.
#[must_use]
pub fn editable(info: &ScheduleInfo) -> bool {
    super::super::is_heartbeat(info)
        && !matches!(info.definition.cadence, Cadence::Unknown)
        && !matches!(info.definition.catch_up, CatchUp::Unknown)
        && !matches!(
            info.state,
            ScheduleState::Unknown | ScheduleState::Ended { reason: EndReason::Unknown, .. }
        )
}

pub(super) fn changed(opened: &ScheduleInfo, rule: &Cadence) -> bool {
    editable(opened) && matches!(rule, Cadence::Cron { .. }) && *rule != opened.definition.cadence
}

pub(super) fn confirmed(
    opened: &ScheduleInfo,
    update: &UpdateScheduleCadence,
    received: &ScheduleInfo,
) -> bool {
    let mut expected = opened.definition.clone();
    expected.cadence = update.cadence.clone();
    received.id == update.schedule
        && received.revision == update.revision.wrapping_add(1)
        && received.defined_by == opened.defined_by
        && received.definition == expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_negotiated_assigned_protocol_offers_the_editor() {
        let assigned = Some(ProtocolVersion::new(48));
        assert!(!admits(None, assigned));
        assert!(!admits(assigned, None));
        assert!(!admits(assigned, Some(ProtocolVersion::new(47))));
        assert!(admits(assigned, assigned));
        assert!(admits(assigned, Some(ProtocolVersion::new(49))));
    }

    #[test]
    fn receipt_requires_same_owner_target_definition_and_revision() {
        let mut opened = crate::schedules::tests::info();
        let session = vessel_proto::SessionId::random();
        opened.definition.target = vessel_proto::automation::ScheduleTarget::Session { session };
        opened.defined_by = vessel_proto::Creator::Agent {
            node: vessel_proto::NodeId::random(),
            session,
        };
        let update = UpdateScheduleCadence {
            schedule: opened.id,
            revision: opened.revision,
            cadence: Cadence::Cron {
                expr: "0 9 * * *".parse().unwrap(),
                zone: "Asia/Seoul".parse().unwrap(),
            },
        };
        let mut received = opened.clone();
        received.revision = update.revision.wrapping_add(1);
        received.definition.cadence = update.cadence.clone();
        assert!(confirmed(&opened, &update, &received));

        let mut reassigned = received.clone();
        reassigned.defined_by = vessel_proto::Creator::Owner;
        assert!(!confirmed(&opened, &update, &reassigned));
        let mut retargeted = received.clone();
        retargeted.definition.target = vessel_proto::automation::ScheduleTarget::Session {
            session: vessel_proto::SessionId::random(),
        };
        assert!(!confirmed(&opened, &update, &retargeted));
        let mut old_revision = received;
        old_revision.revision = opened.revision;
        assert!(!confirmed(&opened, &update, &old_revision));
    }
}
