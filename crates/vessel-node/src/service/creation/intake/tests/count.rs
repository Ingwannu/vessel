//! The intake uses the actual Agents owner at its last available child slot.
use super::*;
use crate::{
    agents::{Agents, MAX_OWNED},
    error::ServiceError,
    principal::Principal,
};
use vessel_proto::{NodeId, Scope};

#[tokio::test]
async fn identical_requests_share_the_real_last_child_count_entry_and_rollback_once() {
    let (_temp, _dir, store, intake) = fixture().await;
    let node = NodeId::random();
    let agents = Agents::new(node);
    let parent = SessionId::random();
    let _secret = agents.reserve(parent, &Principal::Owner, false).unwrap();
    let principal = Principal::Agent {
        node,
        session: parent,
        scopes: vec![Scope::Read, Scope::Operate],
    };
    for _ in 1..MAX_OWNED {
        let _secret = agents
            .reserve(SessionId::random(), &principal, false)
            .unwrap();
    }
    let prepare = |receipt: Receipt| {
        let Resource::Agent { session } = receipt.snapshot().reserved.resource.unwrap() else {
            panic!("agent fixture required");
        };
        std::future::ready(agents.reserve_creation(session, &principal, false))
    };
    let request = || {
        let mut request = candidate("actual-last-slot", 7);
        request.identity =
            CreationIdentity::for_agent(node, parent, Key::parse("actual-last-slot").unwrap());
        request
    };
    let (one, two) = tokio::join!(
        intake.admit(&store, request(), prepare),
        intake.admit(&store, request(), prepare),
    );
    let one = one.unwrap();
    let two = two.unwrap();
    assert!(matches!(
        (&one, &two),
        (Admitted::Start(_), Admitted::Observe(_)) | (Admitted::Observe(_), Admitted::Start(_))
    ));
    assert_eq!(snapshot(&one), snapshot(&two));
    let next = SessionId::random();
    assert!(matches!(
        agents.reserve(next, &principal, false),
        Err(ServiceError::AgentLimit { .. })
    ));
    drop(one);
    drop(two);
    let _secret = agents.reserve(next, &principal, false).unwrap();
    store.close().await;
}
