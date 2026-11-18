//! `session.reload` and its record (TRACKER 803): the wire form, the
//! protocol a call needs, and what an older peer reads instead.

use super::*;
use crate::{
    AgentEvent, EventRecord, EventSeq, HarnessSessionId, Request as _, SessionEvent, SessionId,
};

#[test]
fn the_request_is_its_session() {
    let session = SessionId::random();
    let encoded = serde_json::to_value(ReloadSession { session }).unwrap();
    assert_eq!(encoded, serde_json::json!({ "session": session }));
    assert_eq!(
        serde_json::from_value::<ReloadSession>(encoded).unwrap(),
        ReloadSession { session }
    );
}

#[test]
fn a_reload_call_needs_the_reload_protocol() {
    let call = ReloadSession {
        session: SessionId::random(),
    }
    .into_call();
    assert_eq!(call.since(), RELOAD_PROTOCOL);
}

#[test]
fn an_older_peer_reads_the_record_as_a_log_line() {
    let reload = RELOAD_PROTOCOL.unwrap();
    let older = ProtocolVersion::new(reload.get().checked_sub(1).unwrap());
    for (appended, line) in [
        (0, "vessel: the agent was reloaded"),
        (
            1,
            "vessel: the agent was reloaded; 1 record of its own history was added",
        ),
        (
            3,
            "vessel: the agent was reloaded; 3 records of its own history were added",
        ),
    ] {
        let record = EventRecord {
            seq: EventSeq::new(4),
            at: chrono::DateTime::UNIX_EPOCH,
            event: SessionEvent::Reloaded {
                harness_session: HarnessSessionId::new("conversation"),
                appended,
            },
        };
        assert_eq!(record.for_protocol(reload), None);
        let read = record.for_protocol(older).unwrap();
        assert_eq!(read.seq, record.seq, "no gap in the log");
        assert_eq!(
            read.event,
            SessionEvent::Agent {
                event: AgentEvent::HarnessLog {
                    line: line.to_owned()
                }
            }
        );
    }
}
