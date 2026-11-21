use tokio::sync::oneshot;
use vessel_proto::AgentAuth;

use super::*;
use crate::principal::Principal;

#[tokio::test]
async fn a_replacement_settles_its_credential_after_its_caller_drops() {
    let fixture = Fixture::new().await;
    let created = fixture.runtime.create(fixture.spec()).await.unwrap();
    let info = created.handle.info();
    let id = info.id;
    let agents = &fixture.sessions.agents;
    let secret = agents
        .reserve(id, &Principal::Owner, true)
        .unwrap()
        .unwrap();
    let auth = AgentAuth { session: id, secret };
    let closing = CancellationToken::new();
    let (_principal, registration) = agents.admit(&auth, &closing).unwrap();
    let (disable, secret) = agents.begin_move(id, false).unwrap();
    assert!(secret.is_none());

    let (started, admitted) = oneshot::channel();
    let (ready, permit) = oneshot::channel();
    let sessions = Arc::clone(&fixture.sessions);
    let caller = tokio::spawn(async move {
        sessions
            .finish_replacement(id, disable, async move {
                started.send(()).unwrap();
                permit.await.unwrap();
                Ok(info)
            })
            .await
    });
    admitted.await.unwrap();
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    assert!(!registration.closing().is_cancelled());
    ready.send(()).unwrap();
    fixture.settle().await;
    assert!(registration.closing().is_cancelled());
    assert!(agents.admit(&auth, &closing).is_none());

    created.handle.kill();
    drop(created.durable);
    fixture.stop().await;
}
