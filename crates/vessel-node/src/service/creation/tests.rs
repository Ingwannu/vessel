//! Origin binding survives forwarding; observation grants no creation authority.

use super::*;
use vessel_proto::creation::Key;
use vessel_proto::{DeviceId, NodeId, Origin, OriginBy, Scope, SessionId};

fn request(kind: Kind) -> ObserveCreation {
    ObserveCreation {
        kind,
        key: Key::parse("same-opaque-key").unwrap(),
    }
}
#[test]
fn local_and_delegated_agents_share_only_their_own_origin_namespace() {
    let node = NodeId::random();
    let parent = SessionId::random();
    let local = Principal::Agent {
        node,
        session: parent,
        scopes: vec![Scope::Read, Scope::Operate],
    };
    let delegated = Principal::delegated(
        DeviceId::random(),
        &Scope::ALL,
        Origin {
            node,
            by: OriginBy::Agent {
                session: parent,
                depth: 1,
            },
            scopes: Scope::ALL.to_vec(),
        },
        NodeId::random(),
    );
    let expected = Identity::for_agent(node, parent, Key::parse("same-opaque-key").unwrap());
    for principal in [local, delegated] {
        assert_eq!(
            identity(&principal, request(Kind::Agent)).unwrap(),
            expected
        );
        for kind in [Kind::Workspace, Kind::Terminal] {
            assert_eq!(
                identity(&principal, request(kind)).unwrap_err().code,
                ErrorCode::Forbidden
            );
        }
        for method in [Method::ObserveCreation, Method::SubscribeCreation] {
            assert!(crate::workspace::refuse_agents(&principal, method).is_ok());
        }
    }
    assert_ne!(
        expected,
        identity(&Principal::Owner, request(Kind::Agent)).unwrap()
    );
    let other_origin = Principal::Agent {
        node: NodeId::random(),
        session: parent,
        scopes: vec![Scope::Read],
    };
    assert_ne!(
        expected,
        identity(&other_origin, request(Kind::Agent)).unwrap()
    );
}
#[test]
fn request_supplied_owner_fields_cannot_change_the_admitted_agent_parent() {
    let node = NodeId::random();
    let parent = SessionId::random();
    let agent = Principal::Agent {
        node,
        session: parent,
        scopes: vec![Scope::Read],
    };
    let forged: ObserveCreation = serde_json::from_str(r#"{"kind":"agent","key":"same-opaque-key","owner":"person","parent":"00000000-0000-0000-0000-000000000000"}"#).unwrap();
    assert_eq!(
        identity(&agent, forged).unwrap(),
        Identity::for_agent(node, parent, Key::parse("same-opaque-key").unwrap())
    );
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
    assert!(identity(&reader, request(Kind::Workspace)).is_ok());
    assert!(identity(&Principal::Owner, request(Kind::Terminal)).is_ok());
    assert_eq!(
        identity(&operator, request(Kind::Agent)).unwrap_err().code,
        ErrorCode::Forbidden
    );
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
    assert_eq!(
        identity(&delegated, request(Kind::Agent)).unwrap_err().code,
        ErrorCode::Forbidden
    );
}
