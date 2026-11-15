//! The read-only seam cannot hide a failed final page or substitute a request.
use super::*;
use std::collections::VecDeque;
use vessel_proto::inbox::{InboxCounts, InboxCursor, InboxRead};
use vessel_proto::{EventSeq, PermissionId, SessionId, ToolCallId};

#[derive(Default)]
struct Memory {
    pages: VecDeque<Result<InboxPage, CallError>>,
    entries: VecDeque<Option<InboxEntry>>,
    read: Vec<InboxKey>,
}
impl Source for Memory {
    async fn page(&mut self, _: ListInbox) -> Result<InboxPage, CallError> {
        self.pages.pop_front().expect("a scheduled page")
    }
    async fn live(&mut self, key: InboxKey) -> Result<Option<InboxEntry>, CallError> {
        self.read.push(key);
        Ok(self.entries.pop_front().expect("a scheduled live lookup"))
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
        read: InboxRead::Read,
        item: InboxItem::Permission {
            request: PermissionRequest {
                id: PermissionId::random(),
                tool_call: ToolCallId::new("tool"),
                options: Vec::new(),
            },
            tool: None,
            detail: vessel_proto::inbox::InboxDetail::Complete,
        },
    }
}
fn page(entries: Vec<InboxEntry>, next: Option<InboxCursor>) -> InboxPage {
    InboxPage {
        entries,
        next,
        counts: InboxCounts::default(),
    }
}

#[tokio::test]
async fn a_late_page_failure_has_no_partial_output_result() {
    let mut source = Memory {
        pages: VecDeque::from([
            Ok(page(vec![entry()], Some(InboxCursor::new(8)))),
            Err(CallError::Disconnected),
        ]),
        ..Memory::default()
    };
    assert!(matches!(
        list(&mut source).await,
        Err(SourceError::Node(CallError::Disconnected))
    ));
}
#[tokio::test]
async fn an_empty_middle_page_is_not_completion() {
    let original = entry();
    let mut source = Memory {
        pages: VecDeque::from([
            Ok(page(Vec::new(), Some(InboxCursor::new(8)))),
            Ok(page(vec![original.clone()], None)),
        ]),
        ..Memory::default()
    };
    let rows = list(&mut source).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].source, original.key);
}
#[tokio::test]
async fn recheck_recovers_only_the_exact_source_and_refuses_changed_or_gone_requests() {
    enum Expected {
        Ready,
        Mismatch,
        Gone,
    }
    let original = entry();
    let rows = vec![Row::of(&original).unwrap().unwrap()];
    let mut changed = original.clone();
    let InboxItem::Permission { request, .. } = &mut changed.item else {
        panic!("permission")
    };
    request.id = PermissionId::random();
    for (item, expected) in [
        (Some(original.clone()), Expected::Ready),
        (Some(changed), Expected::Mismatch),
        (None, Expected::Gone),
    ] {
        let mut source = Memory {
            entries: VecDeque::from([item]),
            ..Memory::default()
        };
        let result = preflight(&mut source, &rows).await;
        match expected {
            Expected::Ready => assert!(
                matches!(result.unwrap().as_slice(), [Ready::Permission { source, .. }] if *source == original.key)
            ),
            Expected::Mismatch => assert!(matches!(result, Err(SourceError::Mismatch))),
            Expected::Gone => assert!(matches!(result, Err(SourceError::Gone))),
        }
        assert_eq!(source.read, vec![original.key]);
    }
}

#[test]
fn preflight_retains_typed_peer_failure_without_printing_its_private_message() {
    let canary = "synthetic-private-peer-diagnostic";
    let error = SourceError::from(CallError::Remote(vessel_proto::WireError::new(
        vessel_proto::ErrorCode::Forbidden,
        canary,
    )));
    let SourceError::Node(CallError::Remote(original)) = &error else {
        panic!("original typed failure retained")
    };
    assert_eq!(original.code, vessel_proto::ErrorCode::Forbidden);
    assert!(original.message.contains(canary));
    assert!(!error.to_string().contains(canary));
    assert!(!format!("{error:?}").contains(canary));
    assert!(std::error::Error::source(&error).is_none());
}
