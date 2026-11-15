//! Replacement process spawning is fenced by actual actor/node admission receipts.
use vessel_session::{StartupHooks, StartupPhase, StartupRefusal};
#[cfg(test)]
#[path = "support/runtime.rs"]
#[expect(
    dead_code,
    reason = "shared fake-runtime helpers used by other integration suites"
)]
mod support;
use support::{Fixture, Log, prompt, structured, within};
#[tokio::test]
async fn checkpoint_wake_waits_for_node_admission_and_a_refusal_never_publishes_a_replacement() {
    within(async {
        let (hooks, mut requests) = StartupHooks::channel();
        let fixture = Fixture::startup_hooks(hooks);
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        handle.send(prompt("plain")).await.unwrap();
        log.turn_over().await;
        let _parked = handle.park().await.unwrap();
        assert!(handle.process_lifetime().is_none());
        let call = handle.clone();
        let wake = tokio::spawn(async move { call.wake().await });
        let request = requests.recv().await.unwrap();
        assert_eq!(request.session(), handle.id());
        assert!(request.folder().is_absolute());
        assert!(
            handle.process_lifetime().is_none(),
            "node admission precedes replacement publication"
        );
        request.refuse(StartupRefusal::Unavailable);
        assert!(wake.await.unwrap().is_err());
        assert!(handle.info().parked.is_some());
        assert!(handle.process_lifetime().is_none());
        let call = handle.clone();
        let wake = tokio::spawn(async move { call.wake().await });
        let request = requests.recv().await.unwrap();
        let mut progress = request.grant().unwrap();
        loop {
            match progress.phase() {
                StartupPhase::Registered => break,
                StartupPhase::Granted | StartupPhase::Starting => {
                    let phase = progress.changed().await.unwrap();
                    assert!(matches!(
                        phase,
                        StartupPhase::Starting | StartupPhase::Registered
                    ));
                }
                StartupPhase::NotStarted => panic!("fake provider did not spawn"),
            }
        }
        assert!(handle.process_lifetime().is_some());
        assert!(wake.await.unwrap().unwrap().parked.is_none());
        handle
            .send(prompt("plain after admitted wake"))
            .await
            .unwrap();
        log.next(|event| {
            matches!(
                event,
                vessel_proto::SessionEvent::Agent {
                    event: vessel_proto::AgentEvent::TurnEnded { .. }
                }
            )
        })
        .await;
        fixture.runtime.shutdown().await;
    })
    .await;
}
