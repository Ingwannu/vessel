//! Real framed read routing through the existing shared schedules actor hub.
use super::*;
use crate::testing::{Home, peer, phase, state, within};
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::schedules::{
    history::reader,
    page::{Page, Snapshot},
};

pub(super) fn request(context: ConnectionContext) -> reader::Request {
    selected(context).1
}
fn selected(context: ConnectionContext) -> (reader::Reader, reader::Request) {
    let mut page = Page::default();
    let key = MachineKey::of(context.machine().as_str()).unwrap();
    page.machines(std::slice::from_ref(&key));
    let _admitted = page.admitted(context);
    let ask = page.due(chrono::DateTime::UNIX_EPOCH).remove(0);
    let info: ScheduleInfo = serde_json::from_str(include_str!(
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
    let mut reader = reader::Reader::default();
    let request = reader
        .open(
            &page
                .row(&vessel_app_view::schedules::page::RowKey {
                    machine: key,
                    schedule,
                })
                .unwrap(),
        )
        .unwrap();
    (reader, request)
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
        let context = admission::insert(&mut state, "laptop", &mut peer);
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
        let context = admission::insert(&mut state, "laptop", &mut peer);
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

#[tokio::test]
async fn historical_session_metadata_uses_its_original_read_without_a_live_opener() {
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut peer = phase("controlled session peer", peer::controlled("laptop")).await;
        let context = admission::insert(&mut state, "laptop", &mut peer);
        let (mut reader, history) = selected(context);
        let session = vessel_proto::SessionId::random();
        let at = chrono::DateTime::UNIX_EPOCH;
        assert!(reader.settle(
            &history,
            Ok(vessel_proto::automation::RunList {
                runs: vec![vessel_proto::automation::ScheduleRun {
                    number: vessel_proto::automation::RunNumber::FIRST,
                    at,
                    due_at: at,
                    trigger: vessel_proto::automation::RunTrigger::Due,
                    outcome: vessel_proto::automation::RunOutcome::Started { session },
                }]
            })
        ));
        let request = reader
            .lookup(
                reader.visit().unwrap(),
                vessel_proto::automation::RunNumber::FIRST,
            )
            .unwrap();
        phase(
            "historical session dispatch",
            state.schedule(request.clone()),
        )
        .await;
        let frame = phase(
            "historical session frame",
            peer.requests.as_mut().unwrap().recv(),
        )
        .await
        .unwrap();
        assert_eq!(frame.call, vessel_proto::Call::GetSession(request.input()));
        let mut info = vessel_proto::SessionInfo::new(
            session,
            vessel_proto::SessionSpec::new(
                vessel_proto::HarnessKind::ClaudeCode,
                vessel_proto::SessionMode::Structured {
                    model: None,
                    resume: None,
                    options: vec![],
                },
            ),
            at,
        );
        info.marks.archived_at = Some(at);
        frame
            .answer
            .send(vessel_proto::Outcome::Ok(vessel_proto::Reply::GetSession(
                info.clone(),
            )))
            .unwrap();
        let crate::Event::Schedules(Message::SessionReplied(answer)) =
            phase("historical session receipt", events.recv())
                .await
                .unwrap()
        else {
            panic!("wrong receipt")
        };
        assert_eq!(answer.request, request);
        assert_eq!(answer.result.unwrap(), info);
        assert!(peer.requests.as_mut().unwrap().try_recv().is_err());
    })
    .await;
}
