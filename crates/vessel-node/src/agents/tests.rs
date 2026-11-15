use vessel_proto::DeviceId;

use super::*;

fn agent_of(agents: &Agents, session: SessionId, scopes: Vec<Scope>) -> Principal {
    Principal::Agent {
        node: agents.node,
        session,
        scopes,
    }
}

fn auth(session: SessionId, secret: &AgentSecret) -> AgentAuth {
    AgentAuth {
        session,
        secret: secret.clone(),
    }
}

#[test]
fn a_credential_admits_its_session_only_until_it_ends() {
    let agents = Arc::new(Agents::new(NodeId::random()));
    let node_closing = CancellationToken::new();
    let id = SessionId::random();
    let secret = agents
        .reserve(id, &Principal::Owner, true)
        .unwrap()
        .unwrap();

    let (principal, registration) = agents.admit(&auth(id, &secret), &node_closing).unwrap();
    assert_eq!(
        principal,
        agent_of(&agents, id, vec![Scope::Read, Scope::Operate])
    );
    // Another session's id, or a guessed secret, is refused.
    assert!(
        agents
            .admit(&auth(SessionId::random(), &secret), &node_closing)
            .is_none()
    );
    let forged = AgentSecret::from_bytes([7; 32]);
    assert!(agents.admit(&auth(id, &forged), &node_closing).is_none());

    agents.retire(id);
    assert!(
        registration.closing().is_cancelled(),
        "the agent's connection stays open"
    );
    assert!(agents.admit(&auth(id, &secret), &node_closing).is_none());
    assert!(!node_closing.is_cancelled());
}

#[test]
fn grants_only_shrink() {
    let agents = Agents::new(NodeId::random());
    let viewer = Principal::Device {
        id: DeviceId::random(),
        scopes: vec![Scope::Operate, Scope::Admin],
    };
    let id = SessionId::random();
    let secret = agents.reserve(id, &viewer, true).unwrap().unwrap();
    let (principal, _registration) = Arc::new(agents)
        .admit(&auth(id, &secret), &CancellationToken::new())
        .unwrap();
    let Principal::Agent { scopes, .. } = principal else {
        panic!("{principal:?}");
    };
    // `admin` never; `read` not, since the device has none.
    assert_eq!(scopes, [Scope::Operate]);
}

#[test]
fn agents_start_agents_only_so_deep_and_own_only_so_many() {
    let agents = Agents::new(NodeId::random());
    let scopes = vec![Scope::Read, Scope::Operate];
    let mut parent = SessionId::random();
    let _none = agents.reserve(parent, &Principal::Owner, false).unwrap();
    for _level in 1..=MAX_DEPTH {
        let child = SessionId::random();
        let _none = agents
            .reserve(child, &agent_of(&agents, parent, scopes.clone()), false)
            .unwrap();
        parent = child;
    }
    let error = agents
        .reserve(
            SessionId::random(),
            &agent_of(&agents, parent, scopes.clone()),
            false,
        )
        .unwrap_err();
    assert!(
        matches!(error, ServiceError::AgentDepth { .. }),
        "{error:?}"
    );

    let busy = SessionId::random();
    let _none = agents.reserve(busy, &Principal::Owner, false).unwrap();
    let mut owned = Vec::new();
    for _ in 0..MAX_OWNED {
        let child = SessionId::random();
        let _none = agents
            .reserve(child, &agent_of(&agents, busy, scopes.clone()), false)
            .unwrap();
        owned.push(child);
    }
    let error = agents
        .reserve(
            SessionId::random(),
            &agent_of(&agents, busy, scopes.clone()),
            false,
        )
        .unwrap_err();
    assert!(
        matches!(error, ServiceError::AgentLimit { .. }),
        "{error:?}"
    );
    // An owned session that ends frees its place.
    agents.retire(owned.pop().unwrap());
    assert!(
        agents
            .reserve(SessionId::random(), &agent_of(&agents, busy, scopes), false)
            .is_ok()
    );
}

#[test]
fn a_move_keeps_the_credential_of_the_agent_that_runs() {
    let agents = Arc::new(Agents::new(NodeId::random()));
    let closing = CancellationToken::new();
    let id = SessionId::random();
    let old = agents
        .reserve(id, &Principal::Owner, true)
        .unwrap()
        .unwrap();

    let new = agents.begin_move(id).unwrap().unwrap();
    assert!(matches!(
        agents.begin_move(id),
        Err(ServiceError::MoveRunning(_))
    ));
    assert!(agents.admit(&auth(id, &old), &closing).is_some());
    assert!(agents.admit(&auth(id, &new), &closing).is_some());
    agents.end_move(id, true);
    assert!(agents.admit(&auth(id, &old), &closing).is_none());
    assert!(agents.admit(&auth(id, &new), &closing).is_some());

    let failed = agents.begin_move(id).unwrap().unwrap();
    agents.end_move(id, false);
    assert!(agents.admit(&auth(id, &failed), &closing).is_none());
    assert!(agents.admit(&auth(id, &new), &closing).is_some());
    // No move began: nothing changes.
    agents.end_move(id, true);
    assert!(agents.admit(&auth(id, &new), &closing).is_some());
}

#[test]
fn duplicate_reservation_preserves_the_original_credential_grant_and_registration() {
    let agents = Arc::new(Agents::new(NodeId::random()));
    let closing = CancellationToken::new();
    let id = SessionId::random();
    let secret = agents
        .reserve(id, &Principal::Owner, true)
        .unwrap()
        .unwrap();
    let (original, registration) = agents.admit(&auth(id, &secret), &closing).unwrap();
    let narrowed = Principal::Device {
        id: DeviceId::random(),
        scopes: vec![Scope::Operate],
    };
    let refused = agents.reserve(id, &narrowed, true).unwrap_err();
    assert!(matches!(refused, ServiceError::AgentReserved(same) if same == id));
    assert_eq!(refused.to_wire().code, vessel_proto::ErrorCode::Conflict);
    let (retained, _second_connection) = agents.admit(&auth(id, &secret), &closing).unwrap();
    assert_eq!(retained, original);
    assert_eq!(agents.depth(id), Some(0));
    assert!(!registration.closing().is_cancelled());
    agents.retire(id);
    assert!(registration.closing().is_cancelled());
}

#[test]
fn concurrent_exact_id_admission_at_the_last_child_slot_has_one_count_and_credential() {
    let agents = Arc::new(Agents::new(NodeId::random()));
    let parent = SessionId::random();
    let _parent = agents.reserve(parent, &Principal::Owner, false).unwrap();
    let principal = agent_of(&agents, parent, vec![Scope::Read, Scope::Operate]);
    for _ in 0..MAX_OWNED.saturating_sub(1) {
        let _child = agents
            .reserve(SessionId::random(), &principal, false)
            .unwrap();
    }
    let child = SessionId::random();
    let barrier = std::sync::Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let run = || {
            let _arrival = barrier.wait();
            agents.reserve(child, &principal, true)
        };
        let one = scope.spawn(run);
        let two = scope.spawn(run);
        [one.join().unwrap(), two.join().unwrap()]
    });
    let mut secret = None;
    let mut refused: usize = 0;
    for result in results {
        match result {
            Ok(credential) => {
                assert!(secret.is_none());
                secret = credential;
            }
            Err(ServiceError::AgentReserved(same)) => {
                assert_eq!(same, child);
                refused = refused.saturating_add(1);
            }
            Err(other) => panic!("wrong admission result: {other:?}"),
        }
    }
    assert_eq!(refused, 1);
    assert!(
        agents
            .admit(&auth(child, &secret.unwrap()), &CancellationToken::new())
            .is_some()
    );
    assert!(matches!(
        agents.reserve(SessionId::random(), &principal, false),
        Err(ServiceError::AgentLimit { .. })
    ));
}

#[test]
fn detach_frees_parent_count_without_growing_grants_resetting_depth_or_replacing_credentials() {
    let agents = Arc::new(Agents::new(NodeId::random()));
    let parent = SessionId::random();
    let _secret = agents.reserve(parent, &Principal::Owner, false).unwrap();
    let principal = agent_of(&agents, parent, vec![Scope::Operate]);
    let child = SessionId::random();
    let secret = agents.reserve(child, &principal, true).unwrap().unwrap();
    let closing = CancellationToken::new();
    let (before, registration) = agents.admit(&auth(child, &secret), &closing).unwrap();
    let before_depth = agents.lock().get(&child).unwrap().depth;
    agents.detach(child);
    let entries = agents.lock();
    let entry = entries.get(&child).unwrap();
    assert_eq!(entry.parent, None);
    assert_eq!(entry.depth, before_depth);
    assert_eq!(entry.scopes, vec![Scope::Operate]);
    drop(entries);
    let (after, _second) = agents.admit(&auth(child, &secret), &closing).unwrap();
    assert_eq!(before, after);
    assert!(!registration.closing().is_cancelled());
}
