use super::*;
use vessel_proto::{DeviceId, Origin, OriginBy, Scope, SessionId};
mod guarded;

#[test]
fn public_host_inventory_requires_local_read_without_inheriting_owner() {
    let local = NodeId::random();
    let session = SessionId::random();
    let agent = Principal::Agent {
        node: local,
        session,
        scopes: vec![Scope::Read],
    };
    assert!(admit(&agent, local).is_ok());
    assert!(admit(&Principal::Owner, local).is_ok());
    assert!(
        admit(
            &Principal::Device {
                id: DeviceId::random(),
                scopes: vec![Scope::Read]
            },
            local
        )
        .is_ok()
    );
    for principal in [
        Principal::Agent {
            node: local,
            session,
            scopes: vec![],
        },
        Principal::Agent {
            node: NodeId::random(),
            session,
            scopes: vec![Scope::Read, Scope::Admin],
        },
        Principal::Device {
            id: DeviceId::random(),
            scopes: vec![],
        },
    ] {
        assert_eq!(
            admit(&principal, local).unwrap_err().code,
            ErrorCode::Forbidden
        );
    }
    // Even a local-looking origin is forwarded authority, never a direct local credential.
    for by in [
        OriginBy::Owner,
        OriginBy::Device {
            id: DeviceId::random(),
        },
        OriginBy::Agent { session, depth: 0 },
    ] {
        let delegated = Principal::delegated(
            DeviceId::random(),
            &Scope::ALL,
            Origin {
                node: local,
                by,
                scopes: Scope::ALL.to_vec(),
            },
            NodeId::random(),
        );
        assert_eq!(
            admit(&delegated, local).unwrap_err().code,
            ErrorCode::Forbidden
        );
    }
    assert_eq!(
        super::super::source(&agent).unwrap_err().code,
        ErrorCode::Forbidden
    );
    for method in [
        Method::AddHostRegistration,
        Method::UpdateHostRegistration,
        Method::RemoveHostRegistration,
    ] {
        assert_eq!(agent.permit(method).unwrap_err().code, ErrorCode::Forbidden);
    }
}
