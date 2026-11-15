//! Authority and full routed target are checked before any session mutation.
use super::*;
use crate::principal::Principal;
use vessel_proto::{DeviceId, NodeId, Origin, OriginBy, Scope, SessionId, SessionRef};

#[test]
fn archive_finished_requires_a_person_with_operate_on_the_exact_child_node() {
    let here = NodeId::random();
    let request = ArchiveChildSession {
        parent: SessionRef {
            node: NodeId::random(),
            session: SessionId::random(),
        },
        child: SessionRef {
            node: here,
            session: SessionId::random(),
        },
    };
    assert!(admit_child(&Principal::Owner, here, request).is_ok());
    assert_eq!(
        admit_child(&Principal::Owner, NodeId::random(), request)
            .unwrap_err()
            .code,
        ErrorCode::InvalidRequest
    );
    for scopes in [vec![Scope::Operate], vec![Scope::Read]] {
        let allowed = scopes.contains(&Scope::Operate);
        let device = Principal::Device {
            id: DeviceId::random(),
            scopes: scopes.clone(),
        };
        assert_eq!(admit_child(&device, here, request).is_ok(), allowed);
        let delegated = Principal::delegated(
            DeviceId::random(),
            &scopes,
            Origin {
                node: NodeId::random(),
                by: OriginBy::Owner,
                scopes: scopes.clone(),
            },
            NodeId::random(),
        );
        assert_eq!(admit_child(&delegated, here, request).is_ok(), allowed);
    }
    let scopes = vec![Scope::Read, Scope::Operate];
    let agent = Principal::Agent {
        node: request.parent.node,
        session: request.parent.session,
        scopes: scopes.clone(),
    };
    assert_eq!(
        admit_child(&agent, here, request).unwrap_err().code,
        ErrorCode::Forbidden
    );
    let delegated = Principal::delegated(
        DeviceId::random(),
        &scopes,
        Origin {
            node: request.parent.node,
            by: OriginBy::Agent {
                session: request.parent.session,
                depth: 0,
            },
            scopes: scopes.clone(),
        },
        NodeId::random(),
    );
    assert_eq!(
        admit_child(&delegated, here, request).unwrap_err().code,
        ErrorCode::Forbidden
    );
}
