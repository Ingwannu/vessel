use super::*;
use vessel_proto::{DeviceId, NodeId, Origin, Scope, SessionId};

#[test]
fn only_humans_have_a_checkpoint_owner_and_delegation_keeps_its_full_origin() {
    assert_eq!(
        Principal::Owner.checkpoint_owner().unwrap(),
        CheckpointOwner::LocalOwner
    );
    let device = DeviceId::random();
    let node = NodeId::random();
    let direct = Principal::Device {
        id: device,
        scopes: vec![Scope::Read, Scope::Operate],
    };
    assert_eq!(
        direct.checkpoint_owner().unwrap(),
        CheckpointOwner::Device { device }
    );
    let agent = Principal::Agent {
        node,
        session: SessionId::random(),
        scopes: vec![Scope::Read, Scope::Operate],
    };
    assert_eq!(
        agent.checkpoint_owner().unwrap_err().code,
        ErrorCode::Forbidden
    );
    let origin = Origin {
        node,
        by: OriginBy::Owner,
        scopes: vec![Scope::Read, Scope::Operate],
    };
    let delegated = Principal::Delegated {
        device,
        scopes: vec![Scope::Read, Scope::Operate],
        origin: origin.clone(),
        via: NodeId::random(),
    };
    assert_eq!(
        delegated.checkpoint_owner().unwrap(),
        CheckpointOwner::DelegatedHuman {
            delivering: device,
            origin_node: node,
            origin: CheckpointHumanOrigin::Owner
        }
    );
    let delegated_agent = Principal::Delegated {
        device,
        scopes: vec![Scope::Operate],
        origin: Origin {
            by: OriginBy::Agent {
                session: SessionId::random(),
                depth: 1,
            },
            ..origin
        },
        via: NodeId::random(),
    };
    assert_eq!(
        delegated_agent.checkpoint_owner().unwrap_err().detail,
        Some(ErrorDetail::Checkpoint {
            reason: CheckpointError::Forbidden
        })
    );
}
