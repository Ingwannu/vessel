//! Every reply carrier uses field-level origin adaptation, even when its
//! method and event existed before origin was introduced.

use super::*;
use crate::{
    EventSeq, SessionEvent, SessionImported, TitleOrigin, TransferDetail, TransferId,
    TransferManifest,
};

fn named() -> SessionInfo {
    let mut info = tests::exited();
    info.title = Some("Kept title".parse().unwrap());
    info.title_origin = TitleOrigin::Generated;
    info
}

fn assert_hidden(reply: &Reply) {
    let old = reply.for_protocol(ProtocolVersion::new(9)).unwrap();
    let encoded = serde_json::to_string(&old).unwrap();
    assert!(encoded.contains("Kept title"));
    assert!(!encoded.contains("title_origin"));
}

#[test]
fn all_session_reply_carriers_hide_origin_from_an_older_peer() {
    let constructors: [fn(SessionInfo) -> Reply; 13] = [
        Reply::GetSession,
        Reply::CreateSession,
        Reply::MoveProfile,
        Reply::RenameSession,
        Reply::OpenSideQuestion,
        Reply::ResetSideQuestion,
        Reply::ResumeSession,
        Reply::MarkSession,
        Reply::ParkSession,
        Reply::WakeSession,
        Reply::ReloadSession,
        Reply::ForkSession,
        Reply::RewindSession,
    ];
    for make in constructors {
        assert_hidden(&make(named()));
    }
    assert_hidden(&Reply::ListSessions(vec![named()]));
    let imported = SessionImported {
        harness_session: crate::HarnessSessionId::new("copy"),
        profile: None,
        cwd: std::path::PathBuf::new(),
        written: 0,
        session: Some(Box::new(named())),
    };
    assert_hidden(&Reply::ImportSession(imported));
    let manifest = TransferManifest {
        transfer: TransferId::random(),
        files: std::collections::BTreeMap::new(),
        detail: TransferDetail::Session {
            harness_session: crate::HarnessSessionId::new("copy"),
            cwd: std::path::PathBuf::new(),
            info: Box::new(named()),
            options: Vec::new(),
        },
    };
    assert_hidden(&Reply::ExportSession(manifest.clone()));
    assert_hidden(&Reply::ExportMemory(manifest));
}

#[test]
fn an_old_event_variant_still_adapts_its_new_field_through_pages_and_envelopes() {
    let event = SessionEvent::Renamed {
        title: "Kept title".parse().unwrap(),
        origin: TitleOrigin::Generated,
    };
    let page = crate::EventPage {
        records: vec![crate::EventRecord {
            seq: EventSeq::ZERO,
            at: chrono::DateTime::UNIX_EPOCH,
            event,
        }],
        next: EventSeq::new(1),
        at_end: true,
    };
    let old = page.for_protocol(ProtocolVersion::new(9)).unwrap();
    assert_eq!(old.records[0].seq, EventSeq::ZERO);
    assert!(matches!(
        old.records[0].event,
        SessionEvent::Renamed {
            origin: TitleOrigin::Unknown,
            ..
        }
    ));
    let mut reply = Reply::ReadEvents(page);
    let wrappers: [fn(Forwarded) -> Reply; 3] = [Reply::Forward, Reply::Relay, Reply::Deliver];
    for wrap in wrappers {
        reply = wrap(Forwarded {
            reply: Box::new(reply),
        });
        let adapted = reply.for_protocol(ProtocolVersion::new(9)).unwrap();
        let json = serde_json::to_string(&adapted).unwrap();
        assert!(json.contains("Kept title"));
        assert!(!json.contains("origin"));
    }
}
