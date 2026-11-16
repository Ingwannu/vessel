//! Startup excludes process states and creation authorities without guessing.
use super::*;
use vessel_proto::{Creator, NodeId, RemoteCreator, SessionOrigin};

#[derive(Debug, Clone, Copy)]
enum Exclusion {
    Idle,
    Starting,
    Parked,
    Archived,
    Side,
    Agent,
    Unattended,
    Remote,
    Legacy,
    Deleted,
}

#[tokio::test]
async fn exclusions_remain_manual_even_with_committed_activity() {
    case(async {
        for case in [
            Exclusion::Idle,
            Exclusion::Starting,
            Exclusion::Parked,
            Exclusion::Archived,
            Exclusion::Side,
            Exclusion::Agent,
            Exclusion::Unattended,
            Exclusion::Remote,
            Exclusion::Legacy,
            Exclusion::Deleted,
        ] {
            let (_temp, dir) = data_dir();
            let store = phase("Store open_with_restart_policy", Store::open_with_restart_policy(&dir, StoreConfig::default(), true))
                .await
                .unwrap()
                .store;
            let mut session = info();
            match case {
                Exclusion::Side => session.spec.side_question_of = Some(SessionId::random()),
                Exclusion::Agent => {
                    session.created_by = Creator::Agent {
                        node: NodeId::random(),
                        session: SessionId::random(),
                    }
                }
                Exclusion::Remote => {
                    session.origin = Some(SessionOrigin::Remote {
                        node: NodeId::random(),
                        by: RemoteCreator::Owner,
                    });
                }
                Exclusion::Idle
                | Exclusion::Starting
                | Exclusion::Parked
                | Exclusion::Archived
                | Exclusion::Unattended
                | Exclusion::Legacy
                | Exclusion::Deleted => {}
            }
            let id = session.id;
            let eligibility = if matches!(case, Exclusion::Unattended | Exclusion::Legacy) {
                RestartEligibility::Never
            } else {
                RestartEligibility::Person
            };
            phase("Store create_session_with_restart", store
                .create_session_with_restart(session, eligibility))
                .await
                .unwrap();
            let status_now = if matches!(case, Exclusion::Starting) {
                SessionStatus::Starting
            } else {
                SessionStatus::Running
            };
            let mut events = vec![status(0, status_now)];
            if !matches!(case, Exclusion::Idle) {
                events.push(started(1));
            }
            if matches!(case, Exclusion::Parked) {
                events.push(record(
                    2,
                    SessionEvent::Parked {
                        reason: vessel_proto::parking::ParkReason::Asked,
                    },
                ));
            }
            phase("Store append", store.append(id, events)).await.unwrap();
            if matches!(case, Exclusion::Archived) {
                let _marks = phase("Store mark", store
                    .mark(id, vessel_proto::Mark::Archive, at(3)))
                    .await
                    .unwrap();
            }
            if matches!(case, Exclusion::Deleted) {
                phase("Store append", store
                    .append(id, vec![status(2, SessionStatus::Interrupted)]))
                    .await
                    .unwrap();
                phase("Store delete_session", store.delete_session(id)).await.unwrap();
            }
            phase("Store close", store.close()).await;
            let store = phase("Store open_with_restart_policy", Store::open_with_restart_policy(&dir, StoreConfig::default(), true))
                .await
                .unwrap()
                .store;
            assert_eq!(phase("Store next_restart", store.next_restart()).await.unwrap(), None, "{case:?}");
            phase("Store close", store.close()).await;
        }
    })
    .await;
}

#[tokio::test]
async fn a_local_turn_after_import_keeps_its_local_creator_eligibility() {
    case(async {
        let (_temp, dir) = data_dir();
        let store = phase("Store open_with_restart_policy", Store::open_with_restart_policy(&dir, StoreConfig::default(), true))
            .await
            .unwrap()
            .store;
        let mut session = info();
        session.origin = Some(SessionOrigin::Imported {
            node: NodeId::random(),
            session: SessionId::random(),
        });
        let id = session.id;
        phase("Store create_session_with_restart", store
            .create_session_with_restart(session, RestartEligibility::Person))
            .await
            .unwrap();
        phase("Store append", store
            .append(id, vec![status(0, SessionStatus::Running), started(1)]))
            .await
            .unwrap();
        phase("Store close", store.close()).await;
        let store = phase("Store open_with_restart_policy", Store::open_with_restart_policy(&dir, StoreConfig::default(), true))
            .await
            .unwrap()
            .store;
        assert_eq!(phase("Store next_restart", store.next_restart()).await.unwrap().unwrap().session, id);
        phase("Store close", store.close()).await;
    })
    .await;
}

#[tokio::test]
async fn an_exhausted_stop_generation_is_an_error_not_a_successful_acknowledgement() {
    case(async {
    let (_temp, dir) = data_dir();
    let store = phase("Store open_with_restart_policy", Store::open_with_restart_policy(&dir, StoreConfig::default(), true))
        .await
        .unwrap()
        .store;
    let session = phase("Store working", working(&store)).await;
    phase("Store snapshot", snapshot(&store, session.id)).await;
    phase("Store append", store
        .append(session.id, vec![status(2, SessionStatus::Interrupted)]))
        .await
        .unwrap();
    let database = rusqlite::Connection::open(dir.database_path()).unwrap();
    assert_eq!(database.execute("UPDATE restart_state SET state=json_set(state,'$.generation',?1) WHERE session_id=?2", rusqlite::params![i64::MAX, session.id.to_string()]).unwrap(), 1);
    let result = phase("Store suppress_ended_restart", store
        .suppress_ended_restart(session.id, EventSeq::new(3)))
        .await;
    assert!(
        matches!(result, Err(vessel_store::StoreError::RestartGenerationExhausted(id)) if id == session.id)
    );
    assert_eq!(
        phase("Store restart_generation", store.restart_generation(session.id)).await.unwrap().get(),
        u64::try_from(i64::MAX).unwrap()
    );
    assert!(
        phase("Store next_restart", store.next_restart()).await.unwrap().is_some(),
        "a failed stop cannot claim it durably consumed intent"
    );
    phase("Store close", store.close()).await;
    }).await;
}
