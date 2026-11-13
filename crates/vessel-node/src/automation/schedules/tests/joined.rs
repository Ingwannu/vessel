//! Where the scheduler meets the rest of protocol 11 ("Integration 3
//! (node)"): a session started again takes none of the schedules its
//! agent made in its last run, even when the scheduler never heard that
//! run end.

use super::*;

#[tokio::test]
async fn a_session_started_again_takes_none_of_its_last_runs_schedules() {
    let clock = HandClock::at("2026-01-01T10:00:00Z");
    let (runner, _jobs) = FakeRunner::answering(prompted());
    let node = scheduler(runner, clock).await;
    let session = SessionId::random();
    let agent = Asker {
        by: Creator::Agent {
            node: NodeId::random(),
            session,
        },
        via: Via::Agent,
    };
    let heartbeat = node
        .schedules
        .create(definition(every(600, "2026-01-01T10:00:00Z")), false, agent)
        .await
        .unwrap();
    let persons = node
        .schedules
        .create(
            definition(every(600, "2026-01-01T10:00:00Z")),
            false,
            person(),
        )
        .await
        .unwrap();
    // No end was broadcast (as when the scheduler fell behind the ends'
    // channel): the start again alone removes them, before it answers.
    node.schedules.starting_again(session).await.unwrap();
    assert!(node.schedules.get(heartbeat.id).await.is_err());
    assert!(node.schedules.get(persons.id).await.is_ok());
    assert!(node.store.runs(heartbeat.id).await.unwrap().is_empty());
}
