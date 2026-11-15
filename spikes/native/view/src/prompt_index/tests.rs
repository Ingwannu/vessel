//! A typed in-memory node supplies pages; no transcript or real agent is needed.
use super::*;
use vessel_proto::prompt_index::PromptPreview;

fn target(name: &str) -> IndexTarget {
    IndexTarget {
        machine: MachineKey::from_book(name),
        session: SessionId::random(),
    }
}
fn page(
    ask: &IndexAsk,
    seqs: &[u64],
    next: Option<u64>,
    epoch: PromptIndexEpoch,
) -> PromptIndexPage {
    PromptIndexPage::new(
        ask.target.session,
        epoch,
        seqs.iter()
            .map(|seq| PromptLocation {
                seq: EventSeq::new(*seq),
                timestamp: chrono::DateTime::UNIX_EPOCH,
                preview: PromptPreview::from_text("한글 prompt"),
            })
            .collect(),
        next.map(EventSeq::new),
    )
    .unwrap()
}
fn epoch() -> PromptIndexEpoch {
    PromptIndexEpoch::new(EventSeq::ZERO, None).unwrap()
}

#[test]
fn pages_are_hidden_until_complete_and_duplicate_old_page_replies_cannot_append_twice() {
    let mut index = PromptIndex::default();
    let first = index.refresh(target("one"), None).unwrap();
    let second = index
        .answered(&first, Ok(page(&first, &[1, 3], Some(3), epoch())))
        .unwrap();
    assert!(index.entries().is_none());
    assert_eq!(second.request.after, Some(EventSeq::new(3)));
    assert_eq!(second.request.epoch, Some(epoch()));
    assert!(
        index
            .answered(&first, Ok(page(&first, &[1, 3], Some(3), epoch())))
            .is_none()
    );
    assert!(
        index
            .answered(&second, Ok(page(&second, &[5], None, epoch())))
            .is_none()
    );
    assert_eq!(
        index
            .entries()
            .unwrap()
            .iter()
            .map(|entry| entry.seq.get())
            .collect::<Vec<_>>(),
        [1, 3, 5]
    );
    assert_eq!(index.phase(), IndexPhase::Ready);
}

#[test]
fn session_switch_close_reopen_and_epoch_changes_refuse_old_addresses() {
    let mut index = PromptIndex::default();
    let a = target("one");
    let first = index.refresh(a.clone(), None).unwrap();
    let other = index.refresh(target("two"), None).unwrap();
    assert!(
        index
            .answered(&first, Ok(page(&first, &[1], None, epoch())))
            .is_none()
    );
    index.clear();
    let reopened = index.refresh(a, None).unwrap();
    assert_ne!(first.ticket, reopened.ticket);
    assert!(
        index
            .answered(&other, Err(IndexProblem::Connection))
            .is_none()
    );
    let next = index
        .answered(&reopened, Ok(page(&reopened, &[1], Some(1), epoch())))
        .unwrap();
    let changed = PromptIndexEpoch::new(EventSeq::ZERO, Some(EventSeq::new(2))).unwrap();
    assert!(
        index
            .answered(&next, Ok(page(&next, &[3], None, changed)))
            .is_none()
    );
    assert_eq!(
        index.phase(),
        IndexPhase::Failed(IndexProblem::HistoryChanged)
    );
    assert!(index.entries().is_none());
}

#[test]
fn streaming_refreshes_coalesce_while_each_set_keeps_one_history_revision() {
    let mut index = PromptIndex::default();
    let target = target("one");
    let first = index.refresh(target.clone(), None).unwrap();
    for latest in 0..100 {
        assert!(
            index
                .refresh(target.clone(), Some(EventSeq::new(latest)))
                .is_none()
        );
    }
    let refresh = index
        .answered(&first, Ok(page(&first, &[1], None, epoch())))
        .unwrap();
    assert_ne!(refresh.ticket, first.ticket);
    assert!(index.entries().is_none());
    assert!(
        index
            .answered(&refresh, Ok(page(&refresh, &[1, 99], None, epoch())))
            .is_none()
    );
    assert_eq!(index.entries().unwrap().len(), 2);
    assert!(index.refresh(target, Some(EventSeq::new(99))).is_none());
}

#[test]
fn wrong_sessions_regressing_pages_and_failures_never_expose_a_partial_set() {
    let mut index = PromptIndex::default();
    let ask = index.refresh(target("one"), None).unwrap();
    let mut wrong = ask.clone();
    wrong.target.session = SessionId::random();
    assert!(
        index
            .answered(&ask, Ok(page(&wrong, &[1], None, epoch())))
            .is_none()
    );
    assert_eq!(index.phase(), IndexPhase::Failed(IndexProblem::Invalid));
    let first = index.restart().unwrap();
    let next = index
        .answered(&first, Ok(page(&first, &[3], Some(3), epoch())))
        .unwrap();
    assert!(
        index
            .answered(&next, Ok(page(&next, &[2], None, epoch())))
            .is_none()
    );
    assert_eq!(index.phase(), IndexPhase::Failed(IndexProblem::Invalid));
    for problem in [
        IndexProblem::Connection,
        IndexProblem::Unsupported,
        IndexProblem::Timeout,
        IndexProblem::Busy,
    ] {
        let retry = index.restart().unwrap();
        assert!(index.answered(&retry, Err(problem)).is_none());
        assert_eq!(index.phase(), IndexPhase::Failed(problem));
        assert!(index.entries().is_none());
    }
}

#[test]
fn metadata_can_draw_unloaded_prompts_and_active_position_never_jumps_ahead() {
    let mut index = PromptIndex::default();
    let ask = index.refresh(target("one"), None).unwrap();
    assert!(
        index
            .answered(&ask, Ok(page(&ask, &[3, 10, 40], None, epoch())))
            .is_none()
    );
    let timeline = crate::timeline::Timeline::default();
    let rows = PromptRows::new(index.entries(), &timeline, &[]);
    assert_eq!(rows.len(), 3);
    assert!(!rows.is_empty());
    assert_eq!(rows.get(2).unwrap().seq, EventSeq::new(40));
    assert_eq!(rows.active(EventSeq::ZERO), None);
    assert_eq!(rows.active(EventSeq::new(3)), Some(0));
    assert_eq!(rows.active(EventSeq::new(39)), Some(1));
    assert_eq!(rows.active(EventSeq::new(400)), Some(2));
    assert_eq!(rows.iter().count(), 3);
}

#[test]
fn a_rewind_seen_during_a_read_rejects_the_old_reply_and_old_epoch() {
    let mut index = PromptIndex::default();
    let first = index.refresh(target("one"), None).unwrap();
    let fresh = index.history_changed(EventSeq::new(5)).unwrap();
    assert!(
        index
            .answered(&first, Ok(page(&first, &[1], None, epoch())))
            .is_none()
    );
    assert!(
        index
            .answered(&fresh, Ok(page(&fresh, &[1], None, epoch())))
            .is_none()
    );
    assert_eq!(
        index.phase(),
        IndexPhase::Failed(IndexProblem::HistoryChanged)
    );
    let fresh = index.restart().unwrap();
    let changed = PromptIndexEpoch::new(EventSeq::ZERO, Some(EventSeq::new(5))).unwrap();
    assert!(
        index
            .answered(&fresh, Ok(page(&fresh, &[1, 6], None, changed)))
            .is_none()
    );
    assert_eq!(index.entries().unwrap().len(), 2);
}
