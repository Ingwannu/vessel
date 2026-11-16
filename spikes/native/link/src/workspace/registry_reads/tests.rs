//! Real actor commands and framed RPC responses, without a guard constructor.
use super::{Request, Response};
use crate::{Command, Event, testing::actor::Actor};
use std::time::Duration;
use vessel_app_view::workspace::registry_projection::{
    RegistryProjection, RegistryReadFailure, RegistryReadKey,
};
use vessel_proto::{
    Call, Outcome, Reply,
    registry::{RegistryRevision, RegistrySnapshot},
    workspace::{GitSupport, ProjectList, WorktreeList},
};

fn key(actor: &Actor, model: &mut RegistryProjection) -> RegistryReadKey {
    let key = model.begin(actor.context()).unwrap();
    assert!(
        model
            .finish(&key, Err(RegistryReadFailure::Refused))
            .unwrap()
    );
    key
}
async fn answer(actor: &mut Actor) {
    for step in 0..3 {
        let request = actor.request().await;
        let reply = match (step, request.call) {
            (0, Call::GetRegistry(_)) => Reply::GetRegistry(RegistrySnapshot {
                node: actor.context().node(),
                revision: RegistryRevision::ZERO,
                workspaces: Vec::new(),
                memberships: Vec::new(),
            }),
            (1, Call::ListProjects(_)) => Reply::ListProjects(ProjectList {
                git: GitSupport::Missing,
                projects: Vec::new(),
                more: false,
            }),
            (2, Call::ListWorktrees(_)) => Reply::ListWorktrees(WorktreeList {
                worktrees: Vec::new(),
                more: false,
            }),
            _ => panic!("registry/catalog call order"),
        };
        request.answer.send(Outcome::Ok(reply)).unwrap();
    }
}
async fn response(actor: &mut Actor) -> Response {
    let Event::RegistryRead(response) = actor.next_event().await else {
        panic!("registry observation")
    };
    response
}
async fn settled(response: &mut Response) {
    const SETTLEMENT_DEADLINE: Duration = Duration::from_secs(5);
    let receipt = response
        .take_settlement()
        .expect("actual registry read worker");
    const JOIN_DEADLINE: Duration = Duration::from_secs(6);
    let mut joined = tokio::task::spawn_blocking(move || receipt.wait(SETTLEMENT_DEADLINE));
    match tokio::time::timeout(JOIN_DEADLINE, &mut joined).await {
        Ok(result) => result.unwrap().expect("registry worker returned normally"),
        Err(_elapsed) => {
            joined.abort();
            let _stopped = tokio::time::timeout(JOIN_DEADLINE, &mut joined)
                .await
                .expect("registry settlement waiter cleanup join");
            panic!("registry worker settlement join deadline");
        }
    }
}

async fn read(actor: &mut Actor, key: RegistryReadKey) -> Response {
    actor
        .dispatch(Command::RegistryRead(Request {
            key,
            changed_from: None,
        }))
        .await;
    answer(actor).await;
    let mut response = response(actor).await;
    settled(&mut response).await;
    response
}

#[tokio::test]
async fn registry_read_delivers_real_guard_and_retirement_invalidates_the_held_observation() {
    crate::testing::within(async {
        let mut actor = Actor::new("registry-fixture").await;
        let mut model = RegistryProjection::default();
        let key = key(&actor, &mut model);
        let observed = read(&mut actor, key.clone()).await;
        let guard = observed.receive(|original, guard, result| {
            assert_eq!(original, key);
            assert!(result.is_ok());
            let guard = guard.unwrap();
            assert!(guard.validate(key.context()).is_ok());
            guard
        });
        actor.retire().await;
        assert!(guard.validate(key.context()).is_err());
        actor.close_peer().await;
    })
    .await;
}

#[tokio::test]
async fn queued_registry_data_is_refused_after_actual_retirement_before_receive() {
    crate::testing::within(async {
        let mut actor = Actor::new("registry-fixture").await;
        let mut model = RegistryProjection::default();
        let key = key(&actor, &mut model);
        let observed = read(&mut actor, key).await;
        actor.retire().await;
        observed.receive(|_key, _guard, result| {
            assert_eq!(result, Err(RegistryReadFailure::Offline));
        });
        actor.close_peer().await;
    })
    .await;
}

#[tokio::test]
async fn queued_registry_observations_keep_the_finite_read_budget_until_they_are_dropped() {
    crate::testing::within(async {
        let mut actor = Actor::new("registry-fixture").await;
        let mut model = RegistryProjection::default();
        let mut held = Vec::new();
        for _ in 0..vessel_app_view::workspace::registry_projection::MAX_REGISTRY_READS {
            let key = key(&actor, &mut model);
            held.push(read(&mut actor, key).await);
        }
        let next = key(&actor, &mut model);
        actor
            .dispatch(Command::RegistryRead(Request {
                key: next,
                changed_from: None,
            }))
            .await;
        response(&mut actor).await.receive(|_key, guard, result| {
            assert!(guard.is_none());
            assert_eq!(result, Err(RegistryReadFailure::Refused));
        });
        drop(held.pop());
        let next = key(&actor, &mut model);
        let accepted = read(&mut actor, next).await;
        accepted.receive(|_key, guard, result| {
            assert!(guard.is_some());
            assert!(result.is_ok());
        });
        drop(held);
        actor.close_peer().await;
    })
    .await;
}

#[tokio::test]
async fn retiring_an_actual_pending_registry_read_settles_without_a_peer_reply() {
    crate::testing::within(async {
        let mut actor = Actor::new("registry-fixture").await;
        let mut model = RegistryProjection::default();
        let key = key(&actor, &mut model);
        actor
            .dispatch(Command::RegistryRead(Request {
                key,
                changed_from: None,
            }))
            .await;
        let pending = actor.request().await;
        assert!(matches!(pending.call, Call::GetRegistry(_)));
        actor.retire().await;
        let mut observed = response(&mut actor).await;
        settled(&mut observed).await;
        observed.receive(|_key, _guard, result| {
            assert_eq!(result, Err(RegistryReadFailure::Offline));
        });
        drop(pending);
        actor.close_peer().await;
    })
    .await;
}
