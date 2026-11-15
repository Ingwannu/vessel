//! Real framed read routing through the existing shared schedules actor hub.
use super::*;
use crate::testing::{Home, peer, phase, state, within};
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::schedules::{
    history::reader,
    page::{Page, Snapshot},
};

pub(super) fn request(context: ConnectionContext) -> reader::Request {
    let mut page = Page::default();
    let key = MachineKey::of(context.machine().as_str()).unwrap();
    page.machines(std::slice::from_ref(&key));
    let _admitted = page.admitted(context);
    let ask = page.due(chrono::DateTime::UNIX_EPOCH).remove(0);
    let info: vessel_proto::automation::ScheduleInfo = serde_json::from_str(include_str!(
        "../../../../view/tests/fixtures/schedules/info.json"
    ))
    .unwrap();
    let schedule = info.id;
    page.listed(
        &ask,
        Ok(Snapshot {
            schedules: vec![info],
            scopes: Ok(vec![vessel_proto::Scope::Read]),
            read_at: chrono::DateTime::UNIX_EPOCH,
            projects: Ok(vec![]),
            worktrees: Ok(vec![]),
        }),
    );
    reader::Reader::default()
        .open(
            &page
                .row(&vessel_app_view::schedules::page::RowKey {
                    machine: key,
                    schedule,
                })
                .unwrap(),
        )
        .unwrap()
}
async fn answer(events: &mut tokio::sync::mpsc::Receiver<crate::Event>) -> HistoryAnswer {
    let event = phase("history receipt", events.recv()).await.unwrap();
    let crate::Event::Schedules(Message::HistoryReplied(answer)) = event else {
        panic!("wrong receipt")
    };
    answer
}
#[tokio::test]
async fn a_history_read_keeps_its_actual_typed_frame_and_original_envelope() {
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut peer = phase("controlled history peer", peer::controlled("laptop")).await;
        let context = super::admission::insert(&mut state, "laptop", &mut peer);
        let request = request(context);
        phase("history dispatch", state.schedule(request.clone())).await;
        let frame = phase(
            "history request frame",
            peer.requests.as_mut().unwrap().recv(),
        )
        .await
        .unwrap();
        assert_eq!(frame.call, vessel_proto::Call::ListRuns(request.input()));
        let list = vessel_proto::automation::RunList { runs: vec![] };
        frame
            .answer
            .send(vessel_proto::Outcome::Ok(vessel_proto::Reply::ListRuns(
                list.clone(),
            )))
            .unwrap();
        let answer = answer(&mut events).await;
        assert_eq!(answer.request, request);
        assert_eq!(answer.result.unwrap(), list);
    })
    .await;
}
#[tokio::test]
async fn an_unsent_history_refusal_settles_the_same_family_without_a_write_warning() {
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut peer = phase("history peer", peer::connect("laptop")).await;
        let context = super::admission::insert(&mut state, "laptop", &mut peer);
        let request = request(context);
        let original = state.clients.remove("laptop").unwrap();
        original.retire();
        phase("retired history dispatch", state.schedule(request.clone())).await;
        let answer = answer(&mut events).await;
        assert_eq!(answer.request, request);
        assert_eq!(
            answer.result.unwrap_err().read_failure(),
            Failure::ConnectionChanged
        );
        assert!(peer.calls.lock().unwrap().is_empty());
        assert_eq!(ScheduleError::Deadline.read_failure(), Failure::TimedOut);
    })
    .await;
}
