//! Every Info carrier keeps pending clock data off the wire, including nesting.

use crate::{
    Creator, HarnessKind, HarnessSessionId, ProtocolVersion, Reply, SessionId, SessionImported,
    SessionInfo, SessionMode, SessionRef, SessionSpec, SessionUpdatedAt, TransferDetail,
    TransferId, TransferManifest,
    forward::Forwarded,
    subagents::{DetachDisposition, DetachReceipt, DetachedSession},
};
use chrono::DateTime;

fn info() -> SessionInfo {
    let mut info = SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: vec![],
            },
        ),
        DateTime::UNIX_EPOCH,
    );
    info.updated_at = Some(SessionUpdatedAt::try_from(DateTime::UNIX_EPOCH).unwrap());
    info
}

fn assert_hidden(reply: &Reply) {
    assert!(serde_json::to_string(reply).unwrap().contains("updated_at"));
    let projected = reply.for_protocol(crate::PROTOCOL_VERSION).unwrap();
    assert!(
        !serde_json::to_string(&projected)
            .unwrap()
            .contains("updated_at")
    );
}

fn assert_each_route(mut reply: Reply) {
    assert_hidden(&reply);
    for wrap in [Reply::Forward, Reply::Relay, Reply::Deliver] {
        reply = wrap(Forwarded {
            reply: Box::new(reply),
        });
        assert_hidden(&reply);
    }
}

#[test]
fn every_info_reply_list_subscription_and_nested_route_hides_pending_time() {
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
        assert_each_route(make(info()));
    }
    assert_each_route(Reply::ListSessions(vec![info()]));
    assert_each_route(Reply::Subscribe(crate::Subscribed {
        subscription: crate::SubscriptionId::new(1),
        info: info(),
    }));
    let parent = SessionRef {
        node: crate::NodeId::random(),
        session: SessionId::random(),
    };
    let mut child = info().with_creator(Creator::Agent {
        node: parent.node,
        session: parent.session,
    });
    let receipt = DetachReceipt::new(parent, DateTime::UNIX_EPOCH);
    child.detach_receipt = Some(receipt);
    assert_each_route(Reply::DetachSession(DetachedSession {
        session: Box::new(child),
        receipt,
        disposition: DetachDisposition::Detached,
    }));
}

#[test]
fn session_and_memory_transfers_and_starting_imports_hide_pending_time() {
    let manifest = TransferManifest {
        transfer: TransferId::random(),
        files: std::collections::BTreeMap::new(),
        detail: TransferDetail::Session {
            info: Box::new(info()),
            harness_session: HarnessSessionId::new("fake-session"),
            cwd: "/work".into(),
            options: vec![],
        },
    };
    assert_each_route(Reply::ExportSession(manifest.clone()));
    assert_each_route(Reply::ExportMemory(manifest));
    assert_each_route(Reply::ImportSession(SessionImported {
        harness_session: HarnessSessionId::new("fake-session"),
        profile: None,
        cwd: "/work".into(),
        written: 0,
        session: Some(Box::new(info())),
    }));
}

#[test]
fn synthetic_assignment_keeps_only_supported_time_and_old_wire_defaults_unknown() {
    let current = crate::PROTOCOL_VERSION;
    let assigned = ProtocolVersion::new(current.get().checked_add(1).unwrap());
    let value = info();
    assert!(
        value
            .for_protocol_with_mutation_time(current, Some(assigned))
            .unwrap()
            .updated_at
            .is_none()
    );
    assert_eq!(
        value.for_protocol_with_mutation_time(assigned, Some(assigned)),
        None
    );
    let mut unknown = info();
    unknown.updated_at = None;
    assert!(
        !serde_json::to_string(&unknown)
            .unwrap()
            .contains("updated_at")
    );
    assert_eq!(
        serde_json::from_str::<SessionInfo>(&serde_json::to_string(&unknown).unwrap())
            .unwrap()
            .updated_at,
        None
    );
    assert_eq!(
        serde_json::from_str::<SessionInfo>(&serde_json::to_string(&value).unwrap())
            .unwrap()
            .updated_at,
        value.updated_at
    );
}
