//! The source a notice claims, and what a window shows after asking.

use chrono::DateTime;
use vessel_proto::inbox::{InboxCursor, InboxEntry, InboxItem, InboxKey, InboxRead};
use vessel_proto::notifications::{NotificationClaim, NotificationSource};
use vessel_proto::{EventSeq, SessionId};

use super::*;
use crate::notices::{Notice, NoticeKind};

fn entry(
    cursor: u64,
    session: SessionId,
    seq: u64,
    read: InboxRead,
    item: InboxItem,
) -> InboxEntry {
    InboxEntry {
        cursor: InboxCursor::new(cursor),
        key: InboxKey {
            session,
            seq: EventSeq::new(seq),
        },
        title: None,
        project: None,
        project_name: None,
        at: DateTime::from_timestamp(1_800_000_000, 0).unwrap(),
        read,
        item,
    }
}

fn notice(session: SessionId, kind: NoticeKind) -> Notice {
    Notice {
        kind,
        machine: "lab".to_owned(),
        session,
        title: String::new(),
    }
}

#[test]
fn no_machine_claims_while_the_attention_protocol_is_unassigned() {
    if ATTENTION_NOTIFICATIONS_PROTOCOL.is_none() {
        assert!(!claims(Some(vessel_proto::PROTOCOL_VERSION)));
        assert!(!claims(Some(ProtocolVersion::new(u32::MAX))));
    }
    assert!(!claims(None));
}

#[test]
fn the_source_is_the_newest_unread_entry_of_the_session_that_matches_the_kind() {
    let here = SessionId::random();
    let other = SessionId::random();
    // Newest first, as the node pages them.
    let page = [
        entry(9, other, 40, InboxRead::Unread, InboxItem::Finished),
        entry(8, here, 30, InboxRead::Read, InboxItem::Finished),
        entry(7, here, 20, InboxRead::Unread, InboxItem::Limit),
        entry(6, here, 10, InboxRead::Unread, InboxItem::Finished),
        entry(5, here, 5, InboxRead::Unread, InboxItem::Failed),
    ];
    assert_eq!(
        source(&page, &notice(here, NoticeKind::Finished)),
        Some(NotificationSource::Session {
            key: InboxKey {
                session: here,
                seq: EventSeq::new(20),
            },
            cursor: InboxCursor::new(7),
        })
    );
    // A turn's end never stands for a request, nor a failure for an end.
    assert_eq!(source(&page, &notice(here, NoticeKind::NeedsYou)), None);
    assert_eq!(
        source(&page[4..], &notice(here, NoticeKind::Finished)),
        None
    );
    // Another session's entries are never this one's.
    assert_eq!(
        source(&page[..1], &notice(here, NoticeKind::Finished)),
        None
    );
}

#[test]
fn only_admitted_or_an_entry_past_the_first_page_shows() {
    assert!(shows(Asked::Answered(NotificationClaim::Admitted)));
    // The coordinator's fallback: ADR 0114's path.
    assert!(shows(Asked::NotOnFirstPage));
    for quiet in [
        NotificationClaim::NotRecipient,
        NotificationClaim::Suppressed,
        NotificationClaim::NoAttention,
        NotificationClaim::AlreadyClaimed,
        NotificationClaim::NoPresentClient,
        NotificationClaim::Unknown,
    ] {
        assert!(!shows(Asked::Answered(quiet)), "{quiet:?}");
    }
    assert!(!shows(Asked::Failed));
}
