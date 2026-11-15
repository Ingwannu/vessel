//! Receipt history stays person-only through both local and delegated ingress.

use super::*;
use vessel_proto::{DeviceId, Method, NodeId, Origin, OriginBy, Scope, SessionId};

#[test]
fn an_agent_cannot_observe_receipts_even_with_read_or_through_a_home() {
    let node = NodeId::random();
    let session = SessionId::random();
    let local = Principal::Agent {
        node,
        session,
        scopes: vec![Scope::Read, Scope::Operate],
    };
    let delegated = Principal::delegated(
        DeviceId::random(),
        &Scope::ALL,
        Origin {
            node,
            by: OriginBy::Agent { session, depth: 1 },
            scopes: Scope::ALL.to_vec(),
        },
        NodeId::random(),
    );
    for principal in [local, delegated] {
        assert_eq!(person(&principal).unwrap_err().code, ErrorCode::Forbidden);
        for method in [Method::ObserveCreation, Method::SubscribeCreation] {
            assert!(crate::workspace::refuse_agents(&principal, method).is_err());
        }
    }
}

#[test]
fn a_person_needs_read_scope_and_forwarding_does_not_add_authority() {
    let reader = Principal::Device {
        id: DeviceId::random(),
        scopes: vec![Scope::Read],
    };
    let operator = Principal::Device {
        id: DeviceId::random(),
        scopes: vec![Scope::Operate],
    };
    assert!(person(&reader).is_ok());
    assert!(person(&Principal::Owner).is_ok());
    for method in [Method::ObserveCreation, Method::SubscribeCreation] {
        assert!(reader.permit(method).is_ok());
        assert_eq!(
            operator.permit(method).unwrap_err().code,
            ErrorCode::Forbidden
        );
        assert!(method.forwardable());
    }
    let delegated = Principal::delegated(
        DeviceId::random(),
        &[Scope::Operate],
        Origin {
            node: NodeId::random(),
            by: OriginBy::Owner,
            scopes: Scope::ALL.to_vec(),
        },
        NodeId::random(),
    );
    assert!(person(&delegated).is_ok());
    assert!(delegated.permit(Method::ObserveCreation).is_err());
}
