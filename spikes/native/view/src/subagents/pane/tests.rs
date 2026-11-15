use super::*;
use crate::{devices::MachineKey, timeline::EntryKind};
use vessel_proto::{AgentEvent, ContentBlock, NodeId, SessionEvent, SessionId};

fn query(id: &str) -> ChildQuery {
    ChildQuery {
        target: Target::new(
            crate::connection::ConnectionEpochs::default()
                .admit(MachineKey::from_book("fixture"), NodeId::random())
                .unwrap(),
            SessionId::random(),
        ),
        child: id.parse().unwrap(),
    }
}
fn record(seq: u64, text: &str) -> EventRecord {
    EventRecord {
        seq: EventSeq::new(seq),
        at: chrono::DateTime::UNIX_EPOCH,
        event: SessionEvent::Agent {
            event: AgentEvent::MessageChunk {
                content: ContentBlock::Text { text: text.into() },
            },
        },
    }
}
fn page(records: Vec<EventRecord>, next: u64) -> ProviderChildPage {
    ProviderChildPage {
        records: records.try_into().unwrap(),
        next: EventSeq::new(next),
        at_end: true,
        history: ChildHistory::Complete,
    }
}
#[test]
fn sparse_parent_sequences_reuse_the_canonical_projection_without_fabricated_child_cursors() {
    let mut tickets = Tickets::default();
    let mut pane = ChildPane::new(query("child"));
    let read = pane.read(None, &mut tickets).unwrap();
    assert!(pane.received(
        &read,
        Ok(page(vec![record(4, "one"), record(29, "two")], 45))
    ));
    assert_eq!(pane.timeline().entries()[0].seq, EventSeq::new(4));
    assert_eq!(pane.timeline().entries()[0].through, EventSeq::new(30));
    assert!(
        matches!(&pane.timeline().entries()[0].kind,EntryKind::Message {text} if text=="onetwo")
    );
    assert_eq!(pane.after(), ChildCursor::After(EventSeq::new(45)));
    assert_eq!(pane.before(), Some(ChildCursor::Before(EventSeq::new(4))));
}
#[test]
fn older_pages_rebuild_in_parent_order_without_rewinding_the_live_continuation() {
    let mut tickets = Tickets::default();
    let mut pane = ChildPane::new(query("child"));
    let read = pane.read(None, &mut tickets).unwrap();
    pane.received(&read, Ok(page(vec![record(8, "recent")], 20)));
    let earlier = pane.read(pane.before(), &mut tickets).unwrap();
    pane.received(&earlier, Ok(page(vec![record(2, "older")], 9)));
    assert_eq!(pane.after(), ChildCursor::After(EventSeq::new(20)));
    assert!(
        matches!(&pane.timeline().entries()[0].kind,EntryKind::Message {text} if text=="olderrecent")
    );
}
#[test]
fn changed_overlap_and_late_other_child_answers_do_not_replace_the_pane() {
    let mut tickets = Tickets::default();
    let mut pane = ChildPane::new(query("child"));
    let first = pane.read(None, &mut tickets).unwrap();
    pane.received(&first, Ok(page(vec![record(3, "original")], 4)));
    let next = pane.read(None, &mut tickets).unwrap();
    assert!(!pane.received(&first, Ok(page(vec![], 99))));
    pane.received(&next, Ok(page(vec![record(3, "different")], 4)));
    assert_eq!(pane.failure(), Some(TrackProblem::InvalidReply));
    assert!(
        matches!(&pane.timeline().entries()[0].kind,EntryKind::Message {text} if text=="original")
    );
    let mut other = ChildPane::new(query("other"));
    other.read(None, &mut tickets).unwrap();
    assert!(!other.received(&next, Ok(page(vec![record(9, "wrong child")], 10))));
}

#[test]
fn a_page_outside_its_requested_parent_window_is_refused() {
    let mut tickets = Tickets::default();
    let mut pane = ChildPane::new(query("child"));
    let read = pane
        .read(Some(ChildCursor::Before(EventSeq::new(5))), &mut tickets)
        .unwrap();
    pane.received(&read, Ok(page(vec![record(5, "not before")], 6)));
    assert_eq!(pane.failure(), Some(TrackProblem::InvalidReply));
    assert!(pane.timeline().entries().is_empty());
}
#[test]
fn individually_valid_pages_cannot_grow_retained_history_past_its_byte_budget() {
    const HALF: usize = bounds::MAX_SCAN_BYTES / 2;
    let text = "x".repeat(HALF);
    let mut tickets = Tickets::default();
    let mut pane = ChildPane::new(query("child"));
    let first = pane.read(None, &mut tickets).unwrap();
    pane.received(&first, Ok(page(vec![record(1, &text)], 2)));
    assert_eq!(pane.failure(), None);
    let next = pane.read(Some(pane.after()), &mut tickets).unwrap();
    pane.received(&next, Ok(page(vec![record(3, &text)], 4)));
    assert_eq!(pane.failure(), Some(TrackProblem::RetentionLimit));
    assert_eq!(pane.timeline().entries().len(), 1);
    assert_eq!(pane.after(), ChildCursor::After(EventSeq::new(2)));
}

#[test]
fn projection_moves_once_and_a_prepend_rebuilds_from_retained_source_records() {
    let mut tickets = Tickets::default();
    let mut pane = ChildPane::new(query("child"));
    assert!(pane.take_projection().is_none());
    let first = pane.read(None, &mut tickets).unwrap();
    pane.received(&first, Ok(page(vec![record(8, "recent")], 20)));
    let owned = pane.take_projection().unwrap();
    assert_eq!(owned.entries().len(), 1);
    assert!(pane.take_projection().is_none());
    assert_eq!(pane.records().count(), 1);
    let earlier = pane.read(pane.before(), &mut tickets).unwrap();
    pane.received(&earlier, Ok(page(vec![record(2, "older")], 9)));
    let owned = pane.take_projection().unwrap();
    assert!(
        matches!(&owned.entries()[0].kind, EntryKind::Message { text } if text == "olderrecent")
    );
    assert_eq!(pane.after(), ChildCursor::After(EventSeq::new(20)));
}

#[test]
fn same_child_reopen_rejects_the_old_reply_and_prepend_invalidates_the_painted_revision() {
    let query = query("same child");
    let mut tickets = Tickets::default();
    let mut old = ChildPane::new(query.clone());
    let delayed = old.read(None, &mut tickets).unwrap();
    let mut reopened = ChildPane::new(query);
    let current = reopened.read(None, &mut tickets).unwrap();
    assert!(!reopened.received(&delayed, Ok(page(vec![record(1, "old")], 2))));
    assert!(reopened.received(&current, Ok(page(vec![record(8, "recent")], 20))));
    let painted = reopened.projection_revision().unwrap();
    let earlier = reopened.read(reopened.before(), &mut tickets).unwrap();
    reopened.received(&earlier, Ok(page(vec![record(2, "earlier")], 9)));
    assert_ne!(reopened.projection_revision(), Some(painted));
}
