//! These are real framed client/actor routing tests, not node preview-time or
//! authority proofs. The controlled peer owns/cancels its transport on Drop.
use super::*;
use crate::testing::{Home, peer, phase, state, within};
use vessel_app_view::schedules::editor::{io, preview};
use vessel_proto::automation::{
    Cadence, EverySeconds, PreviewCount, PreviewSchedule, SchedulePreview,
};

fn operation() -> io::Operation {
    io::Operation::Preview(PreviewSchedule {
        cadence: Cadence::Every {
            seconds: EverySeconds::try_from(60).unwrap(),
            anchor: None,
        },
        after: Some(chrono::DateTime::UNIX_EPOCH),
        count: PreviewCount::try_from(preview::NEXT_RUNS).unwrap(),
    })
}
async fn form_answer(events: &mut tokio::sync::mpsc::Receiver<crate::Event>) -> FormAnswer {
    let event = phase("form answer", events.recv()).await.unwrap();
    let crate::Event::Schedules(Message::FormReplied(answer)) = event else {
        panic!("not a form reply")
    };
    answer
}
#[tokio::test]
async fn a_preview_keeps_its_original_envelope_and_actual_typed_request() {
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut peer = phase("controlled peer", peer::controlled("laptop")).await;
        let context = super::admission::insert(&mut state, "laptop", &mut peer);
        let request = io::Request::on(context, Tickets::default().issue().unwrap(), operation());
        phase("form dispatch", state.schedule(request.clone())).await;
        let frame = phase("preview frame", peer.requests.as_mut().unwrap().recv())
            .await
            .unwrap();
        let vessel_proto::Call::PreviewSchedule(sent) = frame.call else {
            panic!("wrong method")
        };
        assert_eq!(request.operation(), &io::Operation::Preview(sent.clone()));
        let projected = SchedulePreview {
            after: chrono::DateTime::UNIX_EPOCH,
            cadence: Cadence::Every {
                seconds: EverySeconds::try_from(60).unwrap(),
                anchor: sent.after,
            },
            runs: [60, 120, 180]
                .map(|seconds| chrono::DateTime::from_timestamp(seconds, 0).unwrap())
                .to_vec(),
        };
        frame
            .answer
            .send(vessel_proto::Outcome::Ok(
                vessel_proto::Reply::PreviewSchedule(projected.clone()),
            ))
            .unwrap();
        let answer = form_answer(&mut events).await;
        assert_eq!(answer.request, request);
        let value = answer.result.unwrap();
        assert_eq!(value, io::Value::Preview(projected));
        let mut mailbox = io::Mailbox::default();
        mailbox.begin(request.clone());
        assert!(mailbox.settle(&request, Ok(value)));
        assert!(mailbox.value().is_some());
    })
    .await;
}
#[tokio::test]
async fn row_and_editor_work_share_one_four_request_pool() {
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut peer = phase("controlled peer", peer::controlled("laptop")).await;
        let context = super::admission::insert(&mut state, "laptop", &mut peer);
        let mut tickets = Tickets::default();
        for index in 0..vessel_app_view::schedules::load::MAX_IN_FLIGHT {
            let ticket = tickets.issue().unwrap();
            if index == 0 {
                phase(
                    "row admission",
                    state.schedule(Request::on(
                        context.clone(),
                        ticket,
                        Ask::Run("00000000-0000-4000-8000-000000000001".parse().unwrap()),
                    )),
                )
                .await;
            } else {
                phase(
                    "form admission",
                    state.schedule(io::Request::on(context.clone(), ticket, operation())),
                )
                .await;
            }
        }
        let busy = io::Request::on(context, tickets.issue().unwrap(), operation());
        phase("bounded refusal", state.schedule(busy.clone())).await;
        let answer = form_answer(&mut events).await;
        assert_eq!(answer.request, busy);
        assert!(matches!(answer.result, Err(ScheduleError::Busy)));
        // Settle every captured request and drain every receipt before dropping
        // the fixture. No detached waiter or silent deadline ends the test.
        for _ in 0..vessel_app_view::schedules::load::MAX_IN_FLIGHT {
            let frame = phase("admitted frame", peer.requests.as_mut().unwrap().recv())
                .await
                .unwrap();
            assert!(matches!(
                frame.call,
                vessel_proto::Call::RunSchedule(_) | vessel_proto::Call::PreviewSchedule(_)
            ));
            frame
                .answer
                .send(vessel_proto::Outcome::Err(WireError::new(
                    ErrorCode::Unsupported,
                    "synthetic refusal",
                )))
                .unwrap();
        }
        let mut rows = 0;
        let mut forms = 0;
        for _ in 0..vessel_app_view::schedules::load::MAX_IN_FLIGHT {
            match phase("settled receipt", events.recv()).await.unwrap() {
                crate::Event::Schedules(Message::Replied(answer)) => {
                    assert!(answer.result.is_err());
                    rows += 1;
                }
                crate::Event::Schedules(Message::FormReplied(answer)) => {
                    assert!(answer.result.is_err());
                    forms += 1;
                }
                event => panic!("wrong receipt: {event:?}"),
            }
        }
        assert_eq!((rows, forms), (1, 3));
    })
    .await;
}
#[test]
fn an_editor_save_lost_after_dispatch_is_uncertain_but_an_unsent_refusal_is_not() {
    use vessel_app_view::schedules::editor::definition::Save;
    let info: vessel_proto::automation::ScheduleInfo = serde_json::from_str(include_str!(
        "../../../../view/tests/fixtures/schedules/info.json"
    ))
    .unwrap();
    let save = io::Operation::Save(Box::new(Save::Create(
        vessel_proto::automation::CreateSchedule {
            definition: info.definition,
            paused: false,
        },
    )));
    let read = operation();
    for error in [
        ScheduleError::Deadline,
        ScheduleError::Retired(vessel_app_view::connection::Refusal::Changed),
        ScheduleError::from(vessel_client::CallError::Disconnected),
    ] {
        assert_eq!(error.editor_failure(&save), Failure::Unconfirmed);
        assert_ne!(error.editor_failure(&read), Failure::Unconfirmed);
    }
    assert_eq!(
        ScheduleError::NotSelected(vessel_app_view::connection::Refusal::Changed)
            .editor_failure(&save),
        Failure::ConnectionChanged
    );
    assert_eq!(ScheduleError::Busy.editor_failure(&save), Failure::Busy);
}
