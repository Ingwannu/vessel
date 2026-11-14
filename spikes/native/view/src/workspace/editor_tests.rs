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
