//! Paseo's `file-panels/source/editor/model.test.ts`, case for case where
//! the steps exist here (the clock and the watch are the panel's).

use vessel_proto::workspace::FileVersion;

use super::{ConflictKind, EditorModel, EditorStatus, Read, Step, Written};

fn version(size: u64) -> FileVersion {
    FileVersion {
        size,
        modified: 0,
        hash: None,
    }
}

/// A model that read `text`.
fn loaded(text: &str) -> EditorModel {
    let mut model = EditorModel::new();
    let first = model.first_read();
    let _step = model.read(first, Read::Text(text.to_owned(), version(1)));
    model
}

/// The write a step asks for: its number and text.
fn write_of(step: &Step) -> (u64, String, bool) {
    match step {
        Step::Write {
            sequence,
            text,
            base,
        } => (*sequence, text.clone(), base.is_some()),
        Step::None | Step::Autosave | Step::StopAutosave | Step::Read { .. } => {
            panic!("expected a write, got {step:?}")
        }
    }
}

fn read_of(step: &Step) -> u64 {
    match step {
        Step::Read { sequence } => *sequence,
        Step::None | Step::Autosave | Step::StopAutosave | Step::Write { .. } => {
            panic!("expected a read, got {step:?}")
        }
    }
}

#[test]
fn loads_the_file_clean() {
    let model = loaded("hello\n");
    assert_eq!(model.status(), EditorStatus::Clean);
    assert_eq!(model.content(), "hello\n");
    assert!(!model.modified());
}

#[test]
fn in_flight_write_cannot_be_replaced_by_autosave_or_flush_after_edit() {
    let mut model = loaded("a");
    let _step = model.edit("first");
    let (first, _, _) = write_of(&model.save());
    assert_eq!(model.edit("second"), Step::Autosave);
    assert_eq!(model.save(), Step::None);
    assert_eq!(model.flush(), Step::None);
    assert_eq!(model.writing(), Some(first));
    assert_eq!(
        model.saved(first, Written::Saved(version(5))),
        Step::Autosave
    );
    let (second, text, _) = write_of(&model.save());
    assert_ne!(first, second);
    assert_eq!(text, "second");
}

#[test]
fn reports_a_file_it_cannot_read() {
    let mut model = EditorModel::new();
    let first = model.first_read();
    let _step = model.read(first, Read::Failed("the disk refused".to_owned()));
    assert_eq!(model.status(), EditorStatus::Failed);
    assert_eq!(model.error(), Some("the disk refused"));
}

#[test]
fn tracks_whether_the_buffer_differs_and_autosaves_the_latest_edit() {
    let mut model = loaded("a");
    assert_eq!(model.edit("ab"), Step::Autosave);
    assert_eq!(model.status(), EditorStatus::Dirty);
    assert_eq!(
        model.edit("a"),
        Step::StopAutosave,
        "back to the file: clean"
    );
    assert_eq!(model.status(), EditorStatus::Clean);
    let _step = model.edit("abc");
    let (_, text, guarded) = write_of(&model.save());
    assert_eq!(text, "abc");
    assert!(guarded, "a save is guarded by what the edit started from");
    assert_eq!(model.status(), EditorStatus::Saving);
}

#[test]
fn keeps_a_newer_edit_modified_when_an_older_save_finishes() {
    let mut model = loaded("a");
    let _step = model.edit("ab");
    let (sequence, _, _) = write_of(&model.save());
    let _step = model.edit("abc");
    assert_eq!(
        model.saved(sequence, Written::Saved(version(2))),
        Step::Autosave
    );
    assert_eq!(model.status(), EditorStatus::Dirty);
    assert!(model.modified());
}

#[test]
fn follows_the_disk_while_clean() {
    let mut model = loaded("a");
    let sequence = read_of(&model.reconcile());
    let _step = model.read(sequence, Read::Text("b".to_owned(), version(1)));
    assert_eq!(model.content(), "b");
    assert_eq!(model.status(), EditorStatus::Clean);
}

#[test]
fn conflicts_when_the_disk_changes_under_a_modified_buffer_and_overwrites() {
    let mut model = loaded("a");
    let _step = model.edit("mine");
    let sequence = read_of(&model.reconcile());
    assert_eq!(
        model.read(sequence, Read::Text("theirs".to_owned(), version(1))),
        Step::StopAutosave
    );
    assert_eq!(model.conflict(), Some((ConflictKind::Changed, true)));
    assert_eq!(model.content(), "mine", "the buffer is kept");
    let (_, text, guarded) = write_of(&model.overwrite());
    assert_eq!(text, "mine");
    assert!(!guarded, "Overwrite writes over what is there");
}

#[test]
fn reload_discards_a_conflicted_buffer_for_the_disk() {
    let mut model = loaded("a");
    let _step = model.edit("mine");
    let sequence = read_of(&model.reconcile());
    let _step = model.read(sequence, Read::Text("theirs".to_owned(), version(1)));
    let sequence = read_of(&model.reload());
    let _step = model.read(sequence, Read::Text("theirs".to_owned(), version(1)));
    assert_eq!(model.content(), "theirs");
    assert_eq!(model.status(), EditorStatus::Clean);
    assert_eq!(model.conflict(), None);
}

#[test]
fn a_save_over_an_unobserved_change_conflicts_instead_of_writing() {
    let mut model = loaded("a");
    let _step = model.edit("mine");
    let (sequence, _, _) = write_of(&model.save());
    assert_eq!(model.saved(sequence, Written::Changed), Step::StopAutosave);
    assert_eq!(model.conflict(), Some((ConflictKind::Changed, true)));
}

#[test]
fn stays_clean_when_the_disk_changes_back_to_its_own_content() {
    let mut model = loaded("a");
    let sequence = read_of(&model.reconcile());
    let _step = model.read(sequence, Read::Text("a".to_owned(), version(1)));
    assert_eq!(model.status(), EditorStatus::Clean);
    let _step = model.edit("ab");
    let sequence = read_of(&model.reconcile());
    let _step = model.read(sequence, Read::Text("a".to_owned(), version(1)));
    assert_eq!(
        model.status(),
        EditorStatus::Dirty,
        "the disk is still what was read"
    );
}

#[test]
fn a_deletion_conflicts_with_local_changes_and_stops_autosave() {
    let mut model = loaded("a");
    let _step = model.edit("mine");
    let sequence = read_of(&model.reconcile());
    assert_eq!(model.read(sequence, Read::Deleted), Step::StopAutosave);
    assert_eq!(model.conflict(), Some((ConflictKind::Deleted, false)));
}

#[test]
fn a_save_over_a_deletion_it_has_not_heard_about_shows_the_deleted_notice() {
    let mut model = loaded("a");
    let _step = model.edit("mine");
    let (sequence, _, _) = write_of(&model.save());
    let _step = model.saved(sequence, Written::Deleted);
    assert_eq!(model.conflict(), Some((ConflictKind::Deleted, false)));
}

#[test]
fn an_external_change_during_a_save_is_met_after_it() {
    let mut model = loaded("a");
    let _step = model.edit("mine");
    let (write, _, _) = write_of(&model.save());
    let sequence = read_of(&model.reconcile());
    assert_eq!(
        model.read(sequence, Read::Text("theirs".to_owned(), version(1))),
        Step::None,
        "held while the write is out"
    );
    let _step = model.saved(write, Written::Saved(version(4)));
    // Saved, the buffer is clean, so it follows the disk (Paseo's rule).
    assert_eq!(model.content(), "theirs");
    assert_eq!(model.status(), EditorStatus::Clean);
}

#[test]
fn reports_failed_saves_without_losing_the_buffer() {
    let mut model = loaded("a");
    let _step = model.edit("mine");
    let (sequence, _, _) = write_of(&model.save());
    let _step = model.saved(sequence, Written::Failed("the disk is full".to_owned()));
    assert_eq!(model.status(), EditorStatus::Error);
    assert_eq!(model.content(), "mine");
    assert_eq!(model.error(), Some("the disk is full"));
    let (_, text, _) = write_of(&model.save());
    assert_eq!(text, "mine", "a failed save is tried again");
}

#[test]
fn flushes_a_modified_buffer_when_left_but_not_a_conflicted_one() {
    let mut model = loaded("a");
    assert_eq!(model.flush(), Step::None, "nothing to keep");
    let _step = model.edit("mine");
    let (_, text, guarded) = write_of(&model.flush());
    assert_eq!(text, "mine");
    assert!(guarded);
    let mut conflicted = loaded("a");
    let _step = conflicted.edit("mine");
    let sequence = read_of(&conflicted.reconcile());
    let _step = conflicted.read(sequence, Read::Text("theirs".to_owned(), version(1)));
    assert_eq!(conflicted.flush(), Step::None);
}

#[test]
fn drops_a_stale_answer() {
    let mut model = loaded("a");
    let _step = model.edit("ab");
    let (old, _, _) = write_of(&model.save());
    let _step = model.edit("abc");
    let _step = model.saved(old, Written::Saved(version(2)));
    let (new, _, _) = write_of(&model.save());
    assert_eq!(
        model.saved(old, Written::Changed),
        Step::None,
        "an old write's answer"
    );
    assert_eq!(model.status(), EditorStatus::Saving);
    let _step = model.saved(new, Written::Saved(version(3)));
    assert_eq!(model.status(), EditorStatus::Clean);
}

#[test]
fn prose_wraps_and_code_keeps_its_lines() {
    assert!(super::wraps("docs/README.md"));
    assert!(super::wraps("docs/한글-문서/설치-안내.Markdown"));
    assert!(!super::wraps("src/main.rs"));
    assert!(
        !super::wraps("notes.md/index.ts"),
        "a folder's name is not the file's"
    );
    assert!(!super::wraps("Makefile"));
}

#[test]
fn saves_original_bom_and_line_separator_while_tracking_normalized_edits() {
    let mut model = loaded("\u{feff}one\r\ntwo\r\n");
    assert_eq!(model.content(), "one\ntwo\n");
    assert_eq!(model.edit("one\r\nchanged\r\n"), Step::Autosave);
    let (sequence, wire, _) = write_of(&model.save());
    assert_eq!(wire, "\u{feff}one\r\nchanged\r\n");
    assert_eq!(
        model.saved(sequence, Written::Saved(version(20))),
        Step::None
    );
    assert!(!model.modified());
    assert_eq!(model.content(), "one\nchanged\n");
    let read = read_of(&model.reconcile());
    assert_eq!(model.read(read, Read::Text(wire, version(20))), Step::None);
    assert_eq!(model.status(), EditorStatus::Clean);
}

#[test]
fn a_queued_disk_read_adopts_its_format_only_after_the_write_settles() {
    let mut model = loaded("a\rb\r");
    let _step = model.edit("new\ntext\n");
    let (sequence, wire, _) = write_of(&model.save());
    assert_eq!(wire, "new\rtext\r");
    let read = read_of(&model.reconcile());
    assert_eq!(
        model.read(read, Read::Text("new\r\ntext\r\n".to_owned(), version(11))),
        Step::None
    );
    let _step = model.saved(sequence, Written::Saved(version(9)));
    assert_eq!(model.status(), EditorStatus::Clean);
    let _step = model.edit("next\ntext\n");
    let (_, wire, _) = write_of(&model.save());
    assert_eq!(wire, "next\r\ntext\r\n");
}

#[test]
fn uncertain_effect_survives_undo_and_fresh_reads_until_original_completion() {
    let mut model = loaded("baseline");
    let _step = model.edit("submitted");
    let (sequence, _, _) = write_of(&model.save());
    let stale_read = read_of(&model.reconcile());
    assert_eq!(
        model.saved(sequence, Written::Uncertain("connection retired".into())),
        Step::StopAutosave
    );
    assert_eq!(model.write_state(), Some(super::WriteState::Uncertain));
    assert!(model.unsettled_base().is_some());
    assert_eq!(model.edit("baseline"), Step::StopAutosave);
    assert!(
        !model.modified(),
        "draft equality says nothing about remote completion"
    );
    assert_eq!(model.status(), EditorStatus::Error);
    assert_eq!(model.save(), Step::None);
    assert_eq!(model.flush(), Step::None);
    assert_eq!(model.overwrite(), Step::None);
    assert_eq!(model.reload(), Step::None);
    assert_eq!(
        model.read(stale_read, Read::Text("old".into(), version(3))),
        Step::None
    );
    let unchanged_read = model.reading();
    assert_eq!(model.reconcile(), Step::None);
    assert_eq!(model.reading(), unchanged_read);
    // Unknown remote effects never issue a recovery read. Even an unsolicited
    // observation of the baseline, submitted bytes or other content is not
    // terminal evidence for the original attempt.
    for observed in ["baseline", "submitted", "third content"] {
        assert_eq!(
            model.read(unchanged_read, Read::Text(observed.into(), version(8))),
            Step::None
        );
        assert_eq!(model.writing(), Some(sequence));
        assert_eq!(model.write_state(), Some(super::WriteState::Uncertain));
        assert_eq!(model.content(), "baseline");
        assert_eq!(model.status(), EditorStatus::Error);
    }
    assert_eq!(model.writing(), Some(sequence));
    assert_eq!(model.write_state(), Some(super::WriteState::Uncertain));
    assert_eq!(model.content(), "baseline");
    assert_eq!(
        model.saved(sequence.saturating_add(1), Written::Saved(version(99))),
        Step::None
    );
    assert_eq!(model.writing(), Some(sequence));
    // A definitive original result can release the obligation. Its submitted
    // text does not overwrite the local undo; a subsequent guarded save is new.
    assert_eq!(
        model.saved(sequence, Written::Saved(version(9))),
        Step::Autosave
    );
    assert_eq!(model.writing(), None);
    assert!(model.modified());
    assert_eq!(model.content(), "baseline");
}

#[test]
fn a_failed_initial_read_can_be_retried_without_discarding_a_loaded_draft() {
    let mut model = EditorModel::new();
    let first = model.first_read();
    let _step = model.read(first, Read::Failed("temporarily unavailable".into()));
    let next = read_of(&model.reconcile());
    assert_ne!(next, first);
    assert_eq!(
        model.read(first, Read::Text("stale".into(), version(5))),
        Step::None
    );
    assert_eq!(model.status(), EditorStatus::Failed);
    assert_eq!(
        model.read(next, Read::Text("recovered".into(), version(9))),
        Step::StopAutosave
    );
    assert_eq!(model.content(), "recovered");
    assert_eq!(model.status(), EditorStatus::Clean);
}

#[test]
fn unverified_conflict_preserves_draft_and_requires_reload_before_overwrite() {
    let mut model = loaded("baseline");
    let _step = model.edit("draft");
    let (sequence, _, _) = write_of(&model.save());
    assert_eq!(
        model.saved(sequence, Written::UnverifiedConflict),
        Step::StopAutosave
    );
    assert_eq!(model.conflict(), Some((ConflictKind::Unverified, false)));
    assert_eq!(model.content(), "draft");
    assert_eq!(model.overwrite(), Step::None);
    assert_eq!(model.save(), Step::None);
    assert!(matches!(model.reload(), Step::Read { .. }));
}

#[test]
fn a_read_started_before_terminal_error_cannot_confirm_the_ended_attempt() {
    let mut model = loaded("baseline");
    let stale = read_of(&model.reconcile());
    let _step = model.edit("submitted");
    let (sequence, _, _) = write_of(&model.save());
    let confirm = read_of(&model.saved(sequence, Written::Ended("node error".into())));
    assert_eq!(
        model.read(stale, Read::Text("submitted".into(), version(2))),
        Step::None
    );
    assert!(model.writing().is_some());
    assert_eq!(model.save(), Step::None);
    assert_eq!(model.reload(), Step::None);
    assert_eq!(
        model.read(confirm, Read::Text("submitted".into(), version(2))),
        Step::StopAutosave
    );
    assert!(model.writing().is_none());
    assert_eq!(model.status(), EditorStatus::Clean);
}

#[test]
fn ended_read_baseline_never_automatically_retries_a_persistent_rejection() {
    let mut model = loaded("baseline");
    let _step = model.edit("draft");
    let (sequence, _, _) = write_of(&model.save());
    let confirm = read_of(&model.saved(sequence, Written::Ended("rejected".into())));
    assert_eq!(model.edit("later draft"), Step::StopAutosave);
    assert_eq!(
        model.read(confirm, Read::Text("baseline".into(), version(3))),
        Step::StopAutosave
    );
    assert_eq!(model.status(), EditorStatus::Error);
    assert_eq!(model.content(), "later draft");
    assert_eq!(model.error(), Some("rejected"));
    assert!(model.writing().is_none());
    assert_eq!(
        model.flush(),
        Step::None,
        "leaving is not a new retry intent"
    );
    assert!(
        matches!(model.save(), Step::Write { .. }),
        "a new explicit save is a new intent"
    );
}

#[test]
fn failed_ended_read_keeps_the_original_cause_until_explicit_confirmation() {
    let mut model = loaded("baseline");
    let _step = model.edit("submitted");
    let (sequence, _, _) = write_of(&model.save());
    let confirm = read_of(&model.saved(sequence, Written::Ended("original error".into())));
    assert_eq!(
        model.read(confirm, Read::Failed("reader offline".into())),
        Step::None
    );
    assert_eq!(model.error(), Some("original error"));
    assert!(model.writing().is_some());
    let retry = read_of(&model.reconcile());
    assert_eq!(
        model.read(retry, Read::Text("third party".into(), version(4))),
        Step::StopAutosave
    );
    assert_eq!(model.conflict(), Some((ConflictKind::Changed, true)));
    assert_eq!(model.content(), "submitted");
    assert!(model.writing().is_none());
}

#[test]
fn an_unconfirmed_running_write_cannot_be_resolved_by_any_fresh_read() {
    let mut model = loaded("baseline");
    let _step = model.edit("submitted");
    let (sequence, _, _) = write_of(&model.save());
    let _step = model.saved(sequence, Written::Uncertain("lost terminal reply".into()));
    assert_eq!(model.reconcile(), Step::None);
    for text in ["baseline", "submitted", "third party"] {
        assert_eq!(
            model.read(model.reading(), Read::Text(text.into(), version(5))),
            Step::None
        );
        assert!(model.writing().is_some());
    }
}

#[test]
fn ended_confirmation_never_replays_later_draft_and_nontext_or_deletion_preserves_it() {
    let mut model = loaded("baseline");
    let _step = model.edit("submitted");
    let (sequence, _, _) = write_of(&model.save());
    let _step = model.edit("later draft");
    let confirm = read_of(&model.saved(sequence, Written::Ended("node error".into())));
    assert_eq!(
        model.read(confirm, Read::Text("submitted".into(), version(6))),
        Step::StopAutosave
    );
    assert_eq!(model.status(), EditorStatus::Dirty);
    assert_eq!(model.content(), "later draft");
    assert!(model.writing().is_none());
    assert_eq!(model.flush(), Step::None);
    assert_eq!(model.edit("new intent"), Step::Autosave);
    assert!(matches!(model.flush(), Step::Write { .. }));
    for (read, kind) in [
        (Read::Deleted, ConflictKind::Deleted),
        (Read::Unverified, ConflictKind::Unverified),
    ] {
        let mut model = loaded("baseline");
        let _step = model.edit("draft");
        let (sequence, _, _) = write_of(&model.save());
        let confirm = read_of(&model.saved(sequence, Written::Ended("node error".into())));
        let _step = model.read(confirm, read);
        assert_eq!(model.conflict(), Some((kind, false)));
        assert_eq!(model.content(), "draft");
        assert!(model.writing().is_none());
    }
}

#[test]
fn input_before_disk_projection_is_preserved_without_automatic_overwrite() {
    let mut model = loaded("shown baseline");
    let read = read_of(&model.reconcile());
    let _step = model.read(read, Read::Text("unseen disk".into(), version(2)));
    assert_eq!(model.content(), "unseen disk");
    assert_eq!(
        model.edit_before_projection("accepted on old display"),
        Step::StopAutosave
    );
    assert_eq!(model.content(), "accepted on old display");
    assert_eq!(model.conflict(), Some((ConflictKind::Changed, true)));
    assert_eq!(model.save(), Step::None);
    assert_eq!(model.flush(), Step::None);
    let _step = model.edit("later local edit");
    let read = read_of(&model.reconcile());
    assert_eq!(
        model.read(read, Read::Text("unseen disk".into(), version(2))),
        Step::StopAutosave
    );
    assert_eq!(model.conflict(), Some((ConflictKind::Changed, true)));
    assert_eq!(model.save(), Step::None);
    assert_eq!(model.flush(), Step::None);
    let read = read_of(&model.reload());
    let _step = model.read(read, Read::Text("unseen disk".into(), version(2)));
    assert_eq!(model.status(), EditorStatus::Clean);
}

#[test]
fn input_before_projection_does_not_settle_an_original_unknown_write() {
    let mut model = loaded("baseline");
    let _step = model.edit("original attempt");
    let (sequence, _, _) = write_of(&model.save());
    let _step = model.saved(sequence, Written::Uncertain("lost reply".into()));
    let _step = model.edit_before_projection("accepted local input");
    assert_eq!(model.writing(), Some(sequence));
    assert_eq!(model.write_state(), Some(super::WriteState::Uncertain));
    assert_eq!(model.content(), "accepted local input");
    assert_eq!(model.save(), Step::None);
    assert_eq!(model.flush(), Step::None);
}
