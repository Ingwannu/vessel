//! A conversation quota read belongs to its actual selected binding epoch.
use crate::connection::ConnectionContext;
use vessel_proto::{EventSeq, SessionId};

/// Rebind records invalidate a pending answer even if the session id stays the same.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSelection {
    /// Parsed book key used by the link registry.
    pub machine: ConnectionContext,
    /// Existing conversation identity.
    pub session: SessionId,
    /// Last observed launch/profile replacement record, when one is known.
    pub binding: Option<EventSeq>,
}
impl SessionSelection {
    /// Retained actions belong to the current reader and the actually open agent,
    /// including its host admission; a pending redraw cannot retarget them.
    #[must_use]
    pub fn admits(
        &self,
        reader: Option<&Self>,
        opened: Option<(&ConnectionContext, SessionId)>,
    ) -> bool {
        reader == Some(self)
            && opened.is_some_and(|(machine, session)| {
                machine == &self.machine && session == self.session
            })
    }
}

/// Records that can replace a launch or change the current provider revoke old replies.
#[must_use]
pub fn binding_epoch(records: &[vessel_proto::EventRecord]) -> Option<EventSeq> {
    use vessel_proto::SessionEvent;
    records
        .iter()
        .filter(|record| {
            matches!(
                record.event,
                SessionEvent::ProfileChanged { .. }
                    | SessionEvent::Woken
                    | SessionEvent::Rewound { .. }
                    | SessionEvent::OptionsChanged { .. }
                    | SessionEvent::Status { .. }
                    | SessionEvent::Parked { .. }
            )
        })
        .map(|record| record.seq)
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::{EventRecord, SessionEvent};
    #[test]
    fn notice_actions_cannot_target_a_new_agent_or_a_reconnected_binding() {
        use crate::connection::{ConnectionEpochs, MachineKey};
        let mut epochs = ConnectionEpochs::default();
        let node = vessel_proto::NodeId::random();
        let machine = epochs
            .admit(MachineKey::from_book("same alias"), node)
            .unwrap();
        let reconnected = epochs
            .admit(MachineKey::from_book("same alias"), node)
            .unwrap();
        let session = SessionId::random();
        let expected = SessionSelection {
            machine: machine.clone(),
            session,
            binding: Some(EventSeq::new(1)),
        };
        let replaced = SessionSelection {
            binding: Some(EventSeq::new(2)),
            ..expected.clone()
        };
        assert!(expected.admits(Some(&expected), Some((&machine, session))));
        assert!(!expected.admits(Some(&expected), Some((&machine, SessionId::random()))));
        assert!(!expected.admits(Some(&expected), Some((&reconnected, session))));
        assert!(!expected.admits(Some(&replaced), Some((&machine, session))));
        assert!(!expected.admits(None, Some((&machine, session))));
        assert!(!expected.admits(Some(&expected), None));
    }
    #[test]
    fn provider_and_profile_changes_replace_the_binding_epoch() {
        let records = [
            EventRecord {
                seq: EventSeq::new(3),
                at: chrono::DateTime::UNIX_EPOCH,
                event: SessionEvent::ProfileChanged {
                    from: None,
                    to: Some("next".parse().unwrap()),
                },
            },
            EventRecord {
                seq: EventSeq::new(7),
                at: chrono::DateTime::UNIX_EPOCH,
                event: SessionEvent::OptionsChanged {
                    options: Vec::new(),
                },
            },
        ];
        assert_eq!(binding_epoch(&records), Some(EventSeq::new(7)));
        assert_eq!(binding_epoch(&[]), None);
    }
}
