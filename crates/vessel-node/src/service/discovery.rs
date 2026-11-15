//! The authenticated source resolves creation inheritance before forwarding.
//! No new origin field, remote lookup or additional authority is introduced.

use vessel_proto::{Call, NodeId, SessionId, WireError};

use super::{NodeService, wire};
use crate::principal::Principal;

impl NodeService {
    /// A remote creator cannot be looked up in this node's session table.
    /// The source has already resolved Background before authenticated delivery.
    pub(super) fn local_creator(&self, principal: &Principal) -> Option<SessionId> {
        local_creator(principal, self.links.node())
    }

    /// Scope/forwardability checks precede this read at the source. Explicit
    /// overrides survive, and a filled field passes the ordinary wire gate.
    pub(crate) async fn inherit_forwarded_listing(
        &self,
        principal: &Principal,
        call: &mut Call,
    ) -> Result<(), WireError> {
        if let Call::CreateSession(request) = call {
            self.sessions
                .inherit_listing(&mut request.spec, self.local_creator(principal))
                .await
                .map_err(|error| wire(&error))?;
        }
        Ok(())
    }
}

fn local_creator(principal: &Principal, here: NodeId) -> Option<SessionId> {
    principal
        .agent()
        .filter(|(node, _)| *node == here)
        .map(|(_, session)| session)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::{DeviceId, Origin, OriginBy, Scope};

    #[test]
    fn only_a_creator_on_this_node_can_be_looked_up_locally() {
        let here = NodeId::random();
        let elsewhere = NodeId::random();
        let session = SessionId::random();
        let agent = Principal::Agent {
            node: here,
            session,
            scopes: vec![Scope::Operate],
        };
        assert_eq!(local_creator(&agent, here), Some(session));
        assert_eq!(local_creator(&agent, elsewhere), None);
        let delegated = Principal::delegated(
            DeviceId::random(),
            &[Scope::Operate],
            Origin {
                node: elsewhere,
                by: OriginBy::Agent { session, depth: 0 },
                scopes: vec![Scope::Operate],
            },
            elsewhere,
        );
        assert_eq!(local_creator(&delegated, here), None);
        assert_eq!(local_creator(&Principal::Owner, here), None);
    }
}
