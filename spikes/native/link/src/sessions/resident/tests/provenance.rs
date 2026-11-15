//! Actual peer payload identity is checked independently of a valid lease/ticket.
use super::*;
use vessel_proto::{Call, GetSession, Mark, Outcome, Reply, conversation::MarkSession};

async fn wrong_reply(fixture: &mut Fixture, mutation: bool) -> SessionInfo {
    let session = fixture.binding.resident.address.session;
    // Clone the same original lease, keeping peer-response mutation separate
    // from the async request's borrow; this does not resolve another client.
    let selected = fixture.selected.clone();
    let request = async {
        if mutation {
            selected
                .call(MarkSession {
                    session,
                    mark: Mark::Archive,
                })
                .await
        } else {
            selected.call(GetSession { session }).await
        }
    };
    tokio::pin!(request);
    let received = phase("observe actual session request", async {
        tokio::select! {
            result = &mut request => {panic!("request completed before reply: {}", result.is_ok());}
            received = fixture.peer.requests.as_mut().unwrap().recv() => received.unwrap(),
        }
    })
    .await;
    let mut info = peer::info(SessionId::random());
    assert_ne!(info.id, session);
    let response = if mutation {
        assert!(matches!(received.call, Call::MarkSession(ask) if ask.session == session));
        info.marks.archived_at = Some(chrono::DateTime::UNIX_EPOCH);
        Reply::MarkSession(info)
    } else {
        assert!(matches!(received.call, Call::GetSession(ask) if ask.session == session));
        Reply::GetSession(info)
    };
    received.answer.send(Outcome::Ok(response)).unwrap();
    phase("decode wrong-session peer reply", request)
        .await
        .unwrap()
}

#[tokio::test]
async fn actual_guarded_wrong_session_metadata_and_completion_never_reach_owner_mutation() {
    within(async {
        for mutation in [false, true] {
            let mut fixture = fixture(true).await;
            let scope =
                Scope::capture(&fixture.selected, &fixture.context, fixture.binding).unwrap();
            let operation = Operation {
                kind: OperationKind::Lifecycle,
                ticket: fixture
                    .residents
                    .admit_operation(fixture.binding, OperationKind::Lifecycle)
                    .unwrap(),
            };
            let info = wrong_reply(&mut fixture, mutation).await;
            let payload = if mutation {
                Payload::Completion {
                    operation,
                    result: Box::new(Ok(Acted::Session(Box::new(info)))),
                }
            } else {
                Payload::Metadata(Box::new(info))
            };
            let (sender, mut receiving) = tokio::sync::mpsc::channel(1);
            phase(
                "queue actual wrong-session reply",
                fixture
                    .selected
                    .send_guarded(&sender, scope.envelope(payload)),
            )
            .await
            .unwrap();
            let envelope = phase("receive actual wrong-session reply", receiving.recv())
                .await
                .unwrap();
            assert_eq!(
                envelope.install(|_, _| panic!("wrong-session payload reached mutation")),
                Err(Problem::WrongSession)
            );
            let retired = fixture
                .residents
                .retire_current(fixture.binding.resident)
                .unwrap()
                .unwrap();
            let [pending] = retired.operations() else {
                panic!("pending admission was consumed by wrong-session result")
            };
            assert_eq!(pending.kind(), operation.kind);
            assert_eq!(pending.ticket(), operation.ticket);
        }
    })
    .await;
}
