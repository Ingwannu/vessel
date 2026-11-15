//! A typed in-memory node returns actual inbox pages, including adversarial cursors.
use super::*;
use vessel_proto::inbox::{
    InboxCount, InboxCounts, InboxEntry, InboxItem, InboxKey, InboxRead, InboxRequest,
};
use vessel_proto::{EventSeq, PermissionId};

fn page(next: Option<InboxCursor>, entries: Vec<InboxEntry>) -> InboxPage {
    InboxPage {
        counts: InboxCounts {
            unread: InboxCount::new(0),
            waiting: InboxCount::new(0),
            total: InboxCount::new(0),
        },
        entries,
        next,
    }
}
fn entry() -> InboxEntry {
    InboxEntry {
        cursor: InboxCursor::new(9),
        key: InboxKey {
            session: SessionId::random(),
            seq: EventSeq::ZERO,
        },
        title: None,
        project: None,
        project_name: None,
        at: chrono::DateTime::UNIX_EPOCH,
        read: InboxRead::Unread,
        item: InboxItem::InSession {
            request: InboxRequest::Permission {
                id: PermissionId::random(),
            },
        },
    }
}

#[test]
fn completion_requires_the_last_page_even_if_an_earlier_page_was_empty() {
    let mut pages = Pages::default();
    assert_eq!(pages.request().unwrap(), Some(ListInbox::default()));
    pages
        .accept(page(Some(InboxCursor::new(8)), Vec::new()))
        .unwrap();
    assert_eq!(
        pages.request().unwrap().unwrap().before,
        Some(InboxCursor::new(8))
    );
    pages.accept(page(None, vec![entry()])).unwrap();
    assert_eq!(pages.request().unwrap(), None);
    assert_eq!(pages.finish().unwrap().len(), 1);
    assert_eq!(Pages::default().finish(), Err(ReadError::Incomplete));
}

#[test]
fn nonadvancing_cursor_poisoning_cannot_turn_into_a_partial_success() {
    let mut pages = Pages::default();
    let before = InboxCursor::new(8);
    pages.accept(page(Some(before), vec![entry()])).unwrap();
    assert_eq!(
        pages.accept(page(Some(before), Vec::new())),
        Err(ReadError::Cursor)
    );
    assert_eq!(pages.request(), Err(ReadError::Cursor));
    assert_eq!(pages.finish(), Err(ReadError::Cursor));
}

#[test]
fn repeated_request_identity_is_not_answered_twice_by_a_batch() {
    let mut pages = Pages::default();
    let original = entry();
    pages
        .accept(page(Some(InboxCursor::new(8)), vec![original.clone()]))
        .unwrap();
    assert_eq!(
        pages.accept(page(None, vec![original])),
        Err(ReadError::Duplicate)
    );
    assert_eq!(pages.finish(), Err(ReadError::Duplicate));
}

#[test]
fn an_unknown_entry_refuses_the_list_and_completion_cannot_accept_more_pages() {
    let mut pages = Pages::default();
    let mut future = entry();
    future.item = InboxItem::Unknown;
    assert_eq!(
        pages.accept(page(None, vec![future])),
        Err(ReadError::Unsupported(UnsupportedKind))
    );
    assert_eq!(pages.finish(), Err(ReadError::Unsupported(UnsupportedKind)));
    let mut complete = Pages::default();
    complete.accept(page(None, Vec::new())).unwrap();
    assert_eq!(
        complete.accept(page(None, Vec::new())),
        Err(ReadError::Finished)
    );
}

#[test]
fn collection_budget_refuses_instead_of_publishing_truncated_results() {
    let mut pages = Pages {
        visited: MAX_PAGES,
        ..Pages::default()
    };
    assert_eq!(pages.accept(page(None, Vec::new())), Err(ReadError::Limit));
    assert_eq!(pages.finish(), Err(ReadError::Limit));
    let mut pages = Pages {
        rows: vec![Row::of(&entry()).unwrap().unwrap(); MAX_ROWS],
        ..Pages::default()
    };
    assert_eq!(
        pages.accept(page(None, vec![entry()])),
        Err(ReadError::Limit)
    );
    assert_eq!(pages.finish(), Err(ReadError::Limit));
}
