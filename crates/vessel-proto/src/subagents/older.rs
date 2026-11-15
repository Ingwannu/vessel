//! Optional parentage is masked after every existing metadata projection.

use crate::{ProtocolVersion, SessionInfo};

impl SessionInfo {
    pub(crate) fn for_protocol_with_parentage(
        &self,
        protocol: ProtocolVersion,
        assigned: Option<ProtocolVersion>,
    ) -> Option<Self> {
        let projected = self.for_protocol_with_names(protocol, crate::GENERATED_NAMES);
        let source = projected.as_ref().unwrap_or(self);
        if source.detach_receipt.is_none() || assigned.is_some_and(|since| since <= protocol) {
            return projected;
        }
        let mut info = projected.unwrap_or_else(|| self.clone());
        info.detach_receipt = None;
        Some(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Creator, HarnessKind, NodeId, SessionId, SessionMode, SessionRef, SessionSpec,
        subagents::DetachReceipt,
    };

    fn detached() -> SessionInfo {
        let parent = SessionRef {
            node: NodeId::random(),
            session: SessionId::random(),
        };
        let mut info = SessionInfo::new(
            SessionId::random(),
            SessionSpec::new(
                HarnessKind::Codex,
                SessionMode::Structured {
                    model: None,
                    resume: None,
                    options: vec![],
                },
            ),
            chrono::DateTime::UNIX_EPOCH,
        )
        .with_creator(Creator::Agent {
            node: parent.node,
            session: parent.session,
        });
        info.detach_receipt = Some(DetachReceipt::new(parent, chrono::DateTime::UNIX_EPOCH));
        info
    }

    #[test]
    fn pending_and_older_peers_never_receive_a_new_parentage_field() {
        let info = detached();
        let assigned = ProtocolVersion::new(24);
        for peer in [ProtocolVersion::new(23), assigned] {
            assert!(
                info.for_protocol_with_parentage(peer, None)
                    .unwrap()
                    .detach_receipt
                    .is_none()
            );
        }
        assert!(
            info.for_protocol_with_parentage(ProtocolVersion::new(23), Some(assigned))
                .unwrap()
                .detach_receipt
                .is_none()
        );
        assert_eq!(
            info.for_protocol_with_parentage(assigned, Some(assigned)),
            None
        );
    }

    #[test]
    fn existing_session_carriers_and_nested_envelopes_use_the_shared_mask() {
        use crate::{Reply, forward::Forwarded};
        let constructors: [fn(SessionInfo) -> Reply; 12] = [
            Reply::GetSession,
            Reply::CreateSession,
            Reply::MoveProfile,
            Reply::RenameSession,
            Reply::OpenSideQuestion,
            Reply::ResetSideQuestion,
            Reply::ResumeSession,
            Reply::MarkSession,
            Reply::ParkSession,
            Reply::WakeSession,
            Reply::ForkSession,
            Reply::RewindSession,
        ];
        for make in constructors {
            let mut reply = make(detached());
            for wrap in [Reply::Forward, Reply::Relay, Reply::Deliver] {
                let adapted = reply.for_protocol(crate::PROTOCOL_VERSION).unwrap();
                let encoded = serde_json::to_string(&adapted).unwrap();
                assert!(!encoded.contains("detach_receipt"));
                assert!(encoded.contains("created_by"));
                reply = wrap(Forwarded {
                    reply: Box::new(reply),
                });
            }
        }
        let list = Reply::ListSessions(vec![detached()]);
        let projected = list.for_protocol(crate::PROTOCOL_VERSION).unwrap();
        assert!(
            !serde_json::to_string(&projected)
                .unwrap()
                .contains("detach_receipt")
        );
    }
}
