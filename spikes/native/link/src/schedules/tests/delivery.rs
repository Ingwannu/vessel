//! Real framed write receipt held between queue delivery and UI installation.
use super::*;
use crate::testing::{Home, peer, phase, state, within};
use vessel_proto::automation::{RunNumber, RunOutcome, RunTrigger, ScheduleRun};

#[derive(Clone, Copy)]
enum AtApply {
    Active,
    Reconnected,
    PhysicallyClosed,
}
fn run() -> ScheduleRun {
    ScheduleRun {
        number: RunNumber::FIRST,
        due_at: chrono::DateTime::UNIX_EPOCH,
        at: chrono::DateTime::UNIX_EPOCH,
        trigger: RunTrigger::Due,
        outcome: RunOutcome::Making,
    }
}
#[tokio::test]
async fn queued_actual_write_is_rechecked_before_apply_without_retargeting() {
    within(async {
        for at_apply in [
            AtApply::Active,
            AtApply::Reconnected,
            AtApply::PhysicallyClosed,
        ] {
            let home = Home::new();
            let (mut state, mut events) = state(&home);
            let mut original = phase("controlled write peer", peer::controlled("laptop")).await;
            let context = admission::insert(&mut state, "laptop", &mut original);
            let request = Request::on(
                context.clone(),
                Tickets::default().issue().unwrap(),
                Ask::Run("00000000-0000-4000-8000-000000000001".parse().unwrap()),
            );
            phase("write dispatch", state.schedule(request.clone())).await;
            let frame = phase(
                "real run request",
                original.requests.as_mut().unwrap().recv(),
            )
            .await
            .unwrap();
            assert!(matches!(frame.call, vessel_proto::Call::RunSchedule(_)));
            frame
                .answer
                .send(vessel_proto::Outcome::Ok(vessel_proto::Reply::RunSchedule(
                    run(),
                )))
                .unwrap();
            // Receipt is queued under the old admission. No Fleet notice is applied.
            let event = phase("queued observed write receipt", events.recv())
                .await
                .unwrap();
            assert!(matches!(
                &event,
                crate::Event::Schedules(Message::Delivered(_))
            ));
            let mut replacement = None;
            match at_apply {
                AtApply::Active => {}
                AtApply::Reconnected => {
                    let old = state.clients.remove("laptop").unwrap();
                    old.retire();
                    drop(old);
                    let mut fresh = phase(
                        "same node new transport",
                        peer::connect_with_node("laptop", context.node()),
                    )
                    .await;
                    let new_context = admission::insert(&mut state, "laptop", &mut fresh);
                    assert_ne!(context.epoch(), new_context.epoch());
                    replacement = Some(fresh);
                }
                AtApply::PhysicallyClosed => {
                    original.task.as_ref().unwrap().abort();
                    let _cancelled =
                        phase("reap original peer", original.task.as_mut().unwrap()).await;
                    let _reaped = original.task.take();
                    phase(
                        "original client physical close",
                        state.clients["laptop"].client.closed(),
                    )
                    .await;
                }
            }
            let crate::Event::Schedules(Message::Replied(answer)) = settled(event) else {
                panic!("original row receipt")
            };
            assert_eq!(answer.request, request);
            match at_apply {
                AtApply::Active => assert!(matches!(answer.result, Ok(Value::Written(_)))),
                AtApply::Reconnected | AtApply::PhysicallyClosed => {
                    let error = answer.result.unwrap_err();
                    assert!(matches!(error, ScheduleError::Retired(_)));
                    assert_eq!(error.failure(request.ask), Failure::Unconfirmed);
                }
            }
            if let Some(fresh) = replacement {
                assert!(fresh.calls.lock().unwrap().is_empty());
            }
        }
    })
    .await;
}
