//! Exercise the real typed client and wire framing, including incomplete
//! pages, stale forms and durable answer confirmation.
use super::{Input, answer, node, peer};
use vessel_app_view::inbox::{Authority, Content, Extent, Key, Pages, Problem};
use vessel_proto::inbox::{
    InboxCount, InboxCounts, InboxCursor, InboxDetail, InboxEntry, InboxItem, InboxKey, InboxPage,
    InboxRead,
};
use vessel_proto::{
    Ack, Call, DeviceSelf, ErrorCode, EventPage, EventSeq, Outcome, PermissionId, PermissionOption,
    PermissionOptionId, PermissionOptionKind, PermissionRequest, Reply, Scope, SessionId,
};

fn entry(cursor: u64) -> InboxEntry {
    InboxEntry {
        cursor: InboxCursor::new(cursor),
        key: InboxKey {
            session: SessionId::random(),
            seq: EventSeq::ZERO,
        },
        title: None,
        project: None,
        project_name: None,
        at: chrono::DateTime::UNIX_EPOCH,
        read: InboxRead::Unread,
        item: InboxItem::Finished,
    }
}
fn page(entries: Vec<InboxEntry>, next: Option<u64>) -> Reply {
    Reply::ListInbox(InboxPage {
        counts: InboxCounts {
            total: InboxCount::new(100),
            waiting: InboxCount::new(80),
            unread: InboxCount::new(90),
        },
        entries,
        next: next.map(InboxCursor::new),
    })
}
fn authority() -> Outcome {
    Outcome::Ok(Reply::GetDeviceSelf(DeviceSelf {
        device: None,
        scopes: vec![Scope::Read],
    }))
}
fn key(client: &super::transport::Client, entry: &InboxEntry) -> Key {
    Key {
        node: client.node(),
        session: entry.key.session,
        seq: entry.key.seq,
    }
}
#[tokio::test]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the test fake rejects unrelated calls rather than copying the canonical method table"
)]
async fn pages_preserve_whole_counts_and_require_explicit_expansion() {
    let latest = entry(10);
    let older = entry(9);
    let peer = peer::connect(move |call| match call {
        Call::GetDeviceSelf(_) => authority(),
        Call::ListInbox(ask) => Outcome::Ok(if ask.before.is_none() {
            page(vec![latest.clone()], Some(10))
        } else {
            assert_eq!(ask.before, Some(InboxCursor::new(10)));
            page(vec![older.clone()], None)
        }),
        other => panic!("unexpected {other:?}"),
    })
    .await;
    let first = node::list(&peer.client, Pages::default()).await.unwrap();
    assert_eq!(first.counts.waiting(), 80);
    assert_eq!(first.entries.len(), 1);
    assert_eq!(first.extent, Extent::More);
    assert_eq!(first.authority, Authority::ReadOnly);
    let expanded = node::list(
        &peer.client,
        Pages::default()
            .more(vessel_app_view::inbox::Position::new(10))
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(expanded.entries.len(), 2);
    assert_eq!(expanded.counts.unread(), 90);
    assert_eq!(expanded.extent, Extent::Complete);
}
#[tokio::test]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the test fake rejects unrelated calls rather than copying the canonical method table"
)]
async fn cursor_cycle_duplicate_and_out_of_order_pages_are_refused() {
    for invalid in 0..3 {
        let latest = entry(10);
        let peer = peer::connect(move |call| match call {
            Call::GetDeviceSelf(_) => authority(),
            Call::ListInbox(_) => Outcome::Ok(match invalid {
                0 => page(vec![latest.clone()], Some(11)),
                1 => page(vec![latest.clone(), latest.clone()], None),
                _ => page(vec![entry(9), entry(10)], None),
            }),
            other => panic!("unexpected {other:?}"),
        })
        .await;
        assert_eq!(
            node::list(&peer.client, Pages::default())
                .await
                .unwrap_err(),
            Problem::InvalidReply
        );
    }
}
#[tokio::test]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the test fake rejects unrelated calls rather than copying the canonical method table"
)]
async fn a_full_get_cannot_substitute_another_sessions_form() {
    let requested = entry(10);
    let replacement = entry(11);
    let peer = peer::connect(move |call| match call {
        Call::GetInboxEntry(ask) => {
            assert_eq!(ask.fetch, vessel_proto::inbox::InboxFetch::LiveRequest);
            Outcome::Ok(Reply::GetInboxEntry(Some(replacement.clone())))
        }
        other => panic!("unexpected {other:?}"),
    })
    .await;
    assert_eq!(
        node::get(&peer.client, key(&peer.client, &requested))
            .await
            .unwrap_err(),
        Problem::InvalidReply
    );
}
#[tokio::test]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the test fake rejects unrelated calls rather than copying the canonical method table"
)]
async fn answer_is_sent_once_and_ack_is_not_confused_with_projection_withdrawal() {
    let id = PermissionId::random();
    let option = PermissionOptionId::new("allow");
    let mut current = entry(10);
    current.item = InboxItem::Permission {
        request: PermissionRequest {
            id,
            tool_call: vessel_proto::ToolCallId::new("tool"),
            options: vec![PermissionOption {
                id: option.clone(),
                name: "Allow".to_owned(),
                kind: PermissionOptionKind::AllowOnce,
            }],
        },
        tool: None,
        detail: InboxDetail::Complete,
    };
    let source = current.clone();
    let counts = std::sync::Arc::new(std::sync::Mutex::new((0, 0)));
    let observed = std::sync::Arc::clone(&counts);
    let mut withdrawn = false;
    let peer = peer::connect(move |call| Outcome::Ok(match call {
        Call::GetInboxEntry(_) => Reply::GetInboxEntry((!withdrawn).then(|| current.clone())),
        Call::GetSession(ask) => {
            assert_eq!(ask.session, current.key.session);
            let mut info = vessel_proto::SessionInfo::new(ask.session, vessel_proto::SessionSpec::new(vessel_proto::HarnessKind::ClaudeCode, vessel_proto::SessionMode::Structured { model: None, resume: None, options: Vec::new() }), chrono::DateTime::UNIX_EPOCH);
            info.next_seq = EventSeq::new(5);
            Reply::GetSession(info)
        }
        Call::SendInput(ask) => {
            assert!(matches!(ask.input, vessel_proto::SessionInput::AnswerPermission { id: got, .. } if got == id));
            observed.lock().unwrap().0 += 1;
            Reply::SendInput(Ack)
        }
        Call::ReadEvents(ask) => {
            assert_eq!(ask.from, EventSeq::new(5));
            observed.lock().unwrap().1 += 1;
            withdrawn = true;
            Reply::ReadEvents(EventPage { records: vec![vessel_proto::EventRecord { seq: EventSeq::new(5), at: chrono::DateTime::UNIX_EPOCH, event: vessel_proto::SessionEvent::Agent { event: vessel_proto::AgentEvent::PermissionResolved { id, outcome: vessel_proto::PermissionOutcome::Cancelled } } }], next: EventSeq::new(6), at_end: true })
        }
        other => panic!("unexpected {other:?}"),
    })).await;
    assert!(
        answer::send(
            &peer.client,
            key(&peer.client, &source),
            Input::Permission(id, option)
        )
        .await
        .unwrap()
        .is_none()
    );
    assert_eq!(*counts.lock().unwrap(), (1, 1));
}
#[tokio::test]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the test fake rejects unrelated calls rather than copying the canonical method table"
)]
async fn stale_answer_is_never_sent_and_remote_text_is_not_exposed() {
    let current = entry(10);
    let source = current.clone();
    let peer = peer::connect(move |call| match call {
        Call::GetInboxEntry(_) => Outcome::Ok(Reply::GetInboxEntry(Some(current.clone()))),
        other => panic!("stale answer escaped validation: {other:?}"),
    })
    .await;
    assert_eq!(
        answer::send(
            &peer.client,
            key(&peer.client, &source),
            Input::Permission(PermissionId::random(), PermissionOptionId::new("allow"))
        )
        .await
        .unwrap_err(),
        Problem::Refused(ErrorCode::Conflict)
    );
    let error = vessel_client::CallError::Remote(vessel_proto::WireError::new(
        ErrorCode::Forbidden,
        "secret-canary",
    ));
    assert_eq!(node::problem(error), Problem::Refused(ErrorCode::Forbidden));
    assert!(matches!(
        node::project(peer.client.node(), source).content,
        Content::Finished
    ));
}

#[tokio::test]
async fn terminal_actions_on_an_older_peer_never_send_a_wire_call() {
    let peer =
        peer::connect(|_call| panic!("a pending/newer terminal method reached an old peer")).await;
    let terminal = vessel_proto::TerminalId::random();
    let revision = vessel_proto::notifications::TerminalAttentionRevision::new(1);
    for action in [
        super::terminals::Action::Claim { terminal, revision },
        super::terminals::Action::Clear { terminal, revision },
    ] {
        assert!(matches!(
            action.run(&peer.client).await,
            Err(Problem::Refused(ErrorCode::Unsupported))
        ));
    }
    assert_eq!(super::terminals::list(&peer.client).await.unwrap(), None);
}
