use super::*;
use crate::schedules::history::reader::tests::{list, run};
use crate::schedules::{load::MachineKey, page::tests::loaded};
use vessel_proto::automation::{RunList, RunOutcome};
use vessel_proto::{SessionId, SessionMode, SessionSpec};
fn ready() -> (Reader, SessionId) {
    let (page, key) = loaded();
    let mut reader = Reader::default();
    let request = reader.open(&page.row(&key).unwrap()).unwrap();
    let session = "00000000-0000-4000-8000-000000000088".parse().unwrap();
    assert!(reader.settle(
        &request,
        Ok(RunList {
            runs: vec![run(1, RunOutcome::Started { session })]
        })
    ));
    (reader, session)
}
fn info(session: SessionId) -> SessionInfo {
    SessionInfo::new(
        session,
        SessionSpec::new(
            vessel_proto::HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: vec![],
            },
        ),
        chrono::DateTime::UNIX_EPOCH,
    )
}
#[test]
fn lookup_is_only_original_archived_session_metadata_not_an_open_action() {
    let (mut reader, session) = ready();
    let visit = reader.visit().unwrap();
    let request = reader.lookup(visit, RunNumber::FIRST).unwrap();
    assert_eq!(request.input(), GetSession { session });
    assert_eq!(request.visit(), visit);
    assert!(reader.lookup(visit, RunNumber::FIRST).is_none());
    let mut archived = info(session);
    archived.marks.archived_at = Some(chrono::DateTime::UNIX_EPOCH);
    assert!(reader.lookup_settled(&request, Ok(archived.clone())));
    let candidate = reader.candidate(visit).unwrap();
    assert_eq!(candidate.request(), &request);
    assert_eq!(candidate.info(), &archived);
    assert!(!reader.lookup_settled(&request, Ok(info(session))));
}
#[test]
fn wrong_id_context_and_late_visit_cannot_produce_an_open_candidate() {
    let (mut reader, session) = ready();
    let visit = reader.visit().unwrap();
    let request = reader.lookup(visit, RunNumber::FIRST).unwrap();
    let mut wrong = request.clone();
    wrong.link.connection =
        crate::schedules::page::tests::rebound(&MachineKey::of("laptop").unwrap());
    assert!(!reader.lookup_settled(&wrong, Ok(info(session))));
    let other = "00000000-0000-4000-8000-000000000099".parse().unwrap();
    assert!(reader.lookup_settled(&request, Ok(info(other))));
    assert_eq!(reader.lookup_failure(), Some(&Failure::Unexpected));
    assert!(reader.candidate(visit).is_none());
    let request = reader.lookup(visit, RunNumber::FIRST).unwrap();
    assert!(reader.connected(&MachineKey::of("laptop").unwrap()));
    assert!(!reader.lookup_settled(&request, Ok(info(session))));
    assert!(reader.candidate(visit).is_none());
    assert!(reader.close(visit));
    assert!(!reader.lookup_settled(&request, Ok(info(session))));
}
#[test]
fn history_refresh_retires_a_background_session_lookup_before_its_reply() {
    let (mut reader, session) = ready();
    let visit = reader.visit().unwrap();
    let old = reader.lookup(visit, RunNumber::FIRST).unwrap();
    let refresh = reader.retry(visit).unwrap();
    assert!(!reader.lookup_settled(&old, Ok(info(session))));
    assert!(reader.settle(&refresh, Ok(list())));
    assert!(reader.candidate(visit).is_none());
    assert!(reader.lookup(visit, RunNumber::FIRST).is_none());
}
