//! Managed delegation and provider-owned tasks are different identities (ADR 0090).
//! Creator is immutable audit data; current parentage applies a durable detach.

pub mod bounds;
mod identity;
mod older;
mod parentage;
mod provider;
mod values;

pub use identity::ProviderChildRef;
pub use parentage::{
    DetachDisposition, DetachReceipt, DetachSession, DetachedSession, InvalidParentage, Parentage,
};
pub use provider::{
    ChildCursor, ChildDescriptor, ChildGap, ChildHistory, ChildLimit, ChildStatus,
    InvalidChildLimit, ListProviderChildren, ReadProviderChild,
};
pub use values::{ProviderChildId, ProviderText, SubagentValueError};

/// The shared gate for detach, provider-child observations, list and timeline.
/// Integration assigns it only after native, CLI and delegated callers consume
/// current parentage without treating a provider task as a managed session.
pub const SUBAGENT_RELATIONS: Option<crate::ProtocolVersion> = None;

#[cfg(test)]
mod pending_tests {
    use super::*;
    use crate::{
        Call, Deliver, Forward, Method, MethodShape, NodeId, Origin, OriginBy, RelayCall, Scope,
        SessionId,
    };

    #[test]
    fn detach_requirement_propagates_through_every_fleet_envelope() {
        let call = Call::DetachSession(DetachSession {
            session: SessionId::random(),
        });
        assert_eq!(call.required_protocol(), SUBAGENT_RELATIONS);
        assert_eq!(Method::DetachSession.since(), SUBAGENT_RELATIONS);
        assert_eq!(Method::DetachSession.scope(), Some(Scope::Operate));
        assert_eq!(Method::DetachSession.shape(), MethodShape::Unary);
        let forwarded = Call::Forward(Forward {
            node: NodeId::random(),
            call: Box::new(call),
        });
        assert_eq!(forwarded.required_protocol(), SUBAGENT_RELATIONS);
        let relayed = Call::Relay(RelayCall {
            node: NodeId::random(),
            call: Box::new(forwarded),
            origin: Origin {
                node: NodeId::random(),
                by: OriginBy::Owner,
                scopes: vec![Scope::Operate],
            },
        });
        assert_eq!(relayed.required_protocol(), SUBAGENT_RELATIONS);
        let delivered = Call::Deliver(Deliver {
            origin: Origin {
                node: NodeId::random(),
                by: OriginBy::Owner,
                scopes: vec![Scope::Operate],
            },
            call: Box::new(relayed),
        });
        assert_eq!(delivered.required_protocol(), SUBAGENT_RELATIONS);
    }
}
