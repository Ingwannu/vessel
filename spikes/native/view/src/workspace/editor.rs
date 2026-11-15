//! A file being edited in the code panel: its buffer, what was last read
//! or saved, the autosave, and what a change on disk does to it (Paseo's
//! `file-panels/source/editor/model.ts`, `FileEditorModel`, ported as a
//! state machine; the panel carries out what each step asks).
//!
//! [Decision Log]
//! - Purpose: edit a file of the session's project in place, saving as
//!   Paseo does (800 ms after the last keystroke), and never write over a
//!   change made on disk that this editor did not see.
//! - Constraint: saved-file observation is pending protocol admission.
//!   A change reaches this model through a guarded save refusal or the
//!   owner's explicit reconcile read, including observation invalidation.
//!   The original read hash guards a save (`save.rs`). Answers come back in
//!   any order, so each write and read carries a number and a stale one is
//!   dropped, as Paseo's sequences do.
//! - Alternatives: a timer polling the file (a read every few seconds for
//!   every open file, and still a window for a lost write); saving only on
//!   a key press (Paseo saves by itself; a person expects it).
//! - Choice: Paseo's statuses and steps, synchronous: each step returns
//!   what the panel must do (`Step`): write, read, start or stop the
//!   autosave. A clean buffer follows the disk; a modified one meets a
//!   change as a conflict, and the person overwrites or reloads.
//! - Cost: the model does not own a watcher or connection. Until the owner
//!   can admit observation, disk changes require a save or explicit reread.

mod reading;
use reading::ConflictRecovery;
mod settled;
mod write_attempt;
pub use write_attempt::WriteState;
use write_attempt::{FlushAfterRecovery, WriteAttempt};

use super::text_format::{TextFormat, normalize_lines};
use vessel_proto::workspace::FileVersion;

/// Paseo's editor statuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorStatus {
    /// The first read is out.
    Loading,
    /// The first read failed.
    Failed,
    /// The buffer is what was last read or saved.
    Clean,
    /// Edited since; the autosave is set.
    Dirty,
    /// A write is out.
    Saving,
    /// The file changed or went on disk while the buffer was edited.
    Conflict,
    /// The last write failed for another reason.
    Error,
}

/// Why the editor is in conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    /// The file on disk is not what this editor last read or saved.
    Changed,
    /// The file is gone.
    Deleted,
    /// A conflict occurred but current editable saved state is unconfirmed.
    Unverified,
}

/// What the panel does after a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Nothing.
    None,
    /// Start the autosave again (800 ms).
    Autosave,
    /// Stop the autosave.
    StopAutosave,
    /// Write `text`, guarded by `base` (`None`: the person's Overwrite);
    /// answer with `saved(sequence, …)`.
    Write {
        /// The write's number.
        sequence: u64,
        /// The buffer as it was when the write left.
        text: String,
        /// What the edit started from.
        base: Option<FileVersion>,
    },
    /// Read the file; answer with `read(sequence, …)`.
    Read {
        /// The read's number.
        sequence: u64,
    },
}

/// How a write ended, as the panel reads the node's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Written {
    /// Written; the file is now this version.
    Saved(FileVersion),
    /// The file on disk is not what the edit started from.
    Changed,
    /// The file is gone.
    Deleted,
    /// Conflict readback did not confirm an existing editable file.
    UnverifiedConflict,
    /// Not written, for another reason (the panel's sentence).
    Failed(String),
    /// Original direct handler ended; request a new read before settling its effect.
    Ended(String),
    /// Possibly sent; only definitive original completion can release its hold.
    Uncertain(String),
}

/// How a read ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Read {
    /// The whole text and its version.
    Text(String, FileVersion),
    /// The file is gone.
    Deleted,
    /// Existing saved content could not be represented as whole editable text.
    Unverified,
    /// Not read (the panel's sentence).
    Failed(String),
}

/// Paseo's `FileEditorModel`, without its clock and its file: see the
/// module's decision log.
#[derive(Debug, Clone)]
pub struct EditorModel {
    status: EditorStatus,
    content: String,
    persisted: String,
    format: TextFormat,
    version: Option<FileVersion>,
    conflict: Option<ConflictKind>,
    conflict_recovery: ConflictRecovery,
    error: Option<String>,
    write_sequence: u64,
    read_sequence: u64,
    /// The write out and the text it carries.
    writing: Option<WriteAttempt>,
    /// A read that came while a write was out, met when it ends.
    read_during_write: Option<Read>,
    /// The read out is a reload: its text replaces the buffer.
    reloading: bool,
    flush_after_recovery: FlushAfterRecovery,
}

/// Paseo's autosave delay.
pub const AUTOSAVE_MS: u64 = 800;

/// The largest file edited; a larger one is shown read-only (Paseo's
/// `MAX_EDITABLE_BYTES`, 1 MiB, the node's largest page).
pub const MAX_EDITABLE_BYTES: u64 = 1024 * 1024;

/// Whether the editor wraps a file's long lines at its width: prose
/// (Markdown) wraps, code keeps each line on one row and scrolls sideways
/// (Paseo's `WRAPPED_EXTENSIONS` in `source/editor/view.web.tsx`).
#[must_use]
pub fn wraps(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.rsplit_once('.').is_some_and(|(_, extension)| {
        extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
    })
}

impl Default for EditorModel {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorModel {
    /// A model whose first read is out (sequence 1).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            status: EditorStatus::Loading,
            content: String::new(),
            persisted: String::new(),
            format: TextFormat::DEFAULT,
            version: None,
            conflict: None,
            conflict_recovery: ConflictRecovery::Automatic,
            error: None,
            write_sequence: 0,
            read_sequence: 1,
            writing: None,
            read_during_write: None,
            reloading: false,
            flush_after_recovery: FlushAfterRecovery::Automatic,
        }
    }

    /// The first read's number.
    #[must_use]
    pub const fn first_read(&self) -> u64 {
        self.read_sequence
    }

    /// Where the editor stands (the toolbar's dot, "Saving…", "Save
    /// failed", the conflict bar).
    #[must_use]
    pub const fn status(&self) -> EditorStatus {
        self.status
    }

    /// The buffer.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Whether the buffer differs from what was last read or saved.
    #[must_use]
    pub fn modified(&self) -> bool {
        self.content != self.persisted
    }

    /// The conflict to show, and whether Overwrite is offered (only for a
    /// modified buffer over a changed file: Paseo's
    /// `getFileConflictCallout`).
    #[must_use]
    pub fn conflict(&self) -> Option<(ConflictKind, bool)> {
        if self.status != EditorStatus::Conflict {
            return None;
        }
        let kind = self.conflict?;
        Some((kind, kind == ConflictKind::Changed && self.modified()))
    }

    /// The write out, by number: the node's answer to a save carries no
    /// number, and one write is out at a time (a save is not made while
    /// one is out), so the answer is this one's.
    #[must_use]
    pub fn writing(&self) -> Option<u64> {
        self.writing.as_ref().map(|attempt| attempt.sequence)
    }

    /// Remote-effect readiness is independent from the draft's text equality.
    #[must_use]
    pub fn write_state(&self) -> Option<WriteState> {
        self.writing.as_ref().map(|attempt| attempt.state)
    }

    /// Guard of the original unsettled write, including explicit overwrite.
    #[must_use]
    pub fn unsettled_base(&self) -> Option<&Option<FileVersion>> {
        self.writing.as_ref().map(|attempt| &attempt.base)
    }

    /// Retire observations without discarding the draft or original write.
    pub fn retire_observations(&mut self) {
        self.read_sequence = self.read_sequence.saturating_add(1);
        self.read_during_write = None;
        self.reloading = false;
    }

    /// The read last asked for; the caller must echo this number with
    /// the response because concurrent reads can finish out of order.
    #[must_use]
    pub const fn reading(&self) -> u64 {
        self.read_sequence
    }

    /// The version last read or saved (its size for the toolbar).
    #[must_use]
    pub const fn version(&self) -> Option<&FileVersion> {
        self.version.as_ref()
    }

    /// The failed read's or write's sentence.
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    const fn loaded(&self) -> bool {
        !matches!(self.status, EditorStatus::Loading | EditorStatus::Failed)
    }

    /// The person typed: the buffer is now `content`.
    pub fn edit(&mut self, content: &str) -> Step {
        let content = normalize_lines(content);
        if !self.loaded() || content == self.content {
            return Step::None;
        }
        self.content = content;
        if matches!(
            self.write_state(),
            Some(WriteState::Uncertain | WriteState::Ended)
        ) {
            self.status = EditorStatus::Error;
            return Step::StopAutosave;
        }
        self.flush_after_recovery = FlushAfterRecovery::Automatic;
        self.error = None;
        if self.status == EditorStatus::Conflict {
            return Step::None;
        }
        if self.modified() {
            self.status = EditorStatus::Dirty;
            Step::Autosave
        } else {
            self.status = EditorStatus::Clean;
            Step::StopAutosave
        }
    }

    /// Preserve accepted input from a control that has not received the model's
    /// newer disk text. The person must resolve that unseen replacement before
    /// another write; an original write obligation remains independently held.
    pub fn edit_before_projection(&mut self, content: &str) -> Step {
        let step = self.edit(content);
        if self.writing.is_none() && self.modified() {
            self.conflict_recovery = ConflictRecovery::Explicit;
            self.enter_conflict(ConflictKind::Changed)
        } else {
            step
        }
    }

    /// The autosave came due, or the person saved.
    pub fn save(&mut self) -> Step {
        if self.writing.is_some()
            || !matches!(self.status, EditorStatus::Dirty | EditorStatus::Error)
        {
            return Step::None;
        }
        self.write(self.version.clone())
    }

    /// Write the buffer over a file that changed on disk.
    pub fn overwrite(&mut self) -> Step {
        if self.writing.is_some()
            || self.conflict != Some(ConflictKind::Changed)
            || self.status != EditorStatus::Conflict
        {
            return Step::None;
        }
        self.write(None)
    }

    /// Replace the buffer with the file on disk (the panel asks first when
    /// the buffer is modified: Paseo's "Reload from disk?").
    pub fn reload(&mut self) -> Step {
        if self.writing.is_some() {
            return Step::None;
        }
        self.read_sequence = self.read_sequence.saturating_add(1);
        self.reloading = true;
        Step::Read {
            sequence: self.read_sequence,
        }
    }

    /// Read the file again to meet a change on disk (the panel's re-read
    /// after a git write): a clean buffer follows it, a modified one
    /// conflicts (Paseo's `reconcile`).
    pub fn reconcile(&mut self) -> Step {
        if (!self.loaded() && self.status != EditorStatus::Failed)
            || self.write_state() == Some(WriteState::Uncertain)
        {
            return Step::None;
        }
        self.read_sequence = self.read_sequence.saturating_add(1);
        self.reloading = false;
        Step::Read {
            sequence: self.read_sequence,
        }
    }

    /// Leaving the file: a modified buffer not in conflict is written
    /// first, guarded (Paseo's `dispose({ flush: true })`).
    pub fn flush(&mut self) -> Step {
        if self.writing.is_none()
            && self.flush_after_recovery == FlushAfterRecovery::Automatic
            && self.modified()
            && self.loaded()
            && self.status != EditorStatus::Conflict
        {
            self.write(self.version.clone())
        } else {
            Step::None
        }
    }

    fn write(&mut self, base: Option<FileVersion>) -> Step {
        self.conflict_recovery = ConflictRecovery::Automatic;
        self.flush_after_recovery = FlushAfterRecovery::Automatic;
        self.write_sequence = self.write_sequence.saturating_add(1);
        let sequence = self.write_sequence;
        self.writing = Some(WriteAttempt {
            sequence,
            text: self.content.clone(),
            base: base.clone(),
            state: WriteState::Pending,
        });
        self.read_during_write = None;
        self.status = EditorStatus::Saving;
        self.conflict = None;
        self.error = None;
        Step::Write {
            sequence,
            text: self.format.encode(&self.content),
            base,
        }
    }

    /// A write's answer.
    pub fn saved(&mut self, sequence: u64, written: Written) -> Step {
        if let Written::Uncertain(sentence) = &written {
            let Some(attempt) = self
                .writing
                .as_mut()
                .filter(|attempt| attempt.sequence == sequence)
            else {
                return Step::None;
            };
            attempt.state = WriteState::Uncertain;
            self.status = EditorStatus::Error;
            self.error = Some(sentence.clone());
            self.read_during_write = None;
            self.read_sequence = self.read_sequence.saturating_add(1);
            return Step::StopAutosave;
        }
        if let Written::Ended(sentence) = &written {
            let Some(attempt) = self
                .writing
                .as_mut()
                .filter(|attempt| attempt.sequence == sequence)
            else {
                return Step::None;
            };
            attempt.state = WriteState::Ended;
            self.flush_after_recovery = FlushAfterRecovery::NewIntentRequired;
            self.status = EditorStatus::Error;
            self.error = Some(sentence.clone());
            self.read_during_write = None;
            self.read_sequence = self.read_sequence.saturating_add(1);
            return Step::Read {
                sequence: self.read_sequence,
            };
        }
        let Some(WriteAttempt { text, .. }) =
            self.writing.take_if(|attempt| attempt.sequence == sequence)
        else {
            return Step::None;
        };
        let step = match written {
            Written::Saved(version) => {
                self.persisted = text;
                self.version = Some(version);
                if self.modified() {
                    self.status = EditorStatus::Dirty;
                    Step::Autosave
                } else {
                    self.status = EditorStatus::Clean;
                    Step::None
                }
            }
            Written::Uncertain(_) | Written::Ended(_) => Step::StopAutosave,
            Written::UnverifiedConflict => self.enter_conflict(ConflictKind::Unverified),
            Written::Changed => self.enter_conflict(ConflictKind::Changed),
            Written::Deleted => self.enter_conflict(ConflictKind::Deleted),
            Written::Failed(sentence) => {
                self.status = EditorStatus::Error;
                self.error = Some(sentence);
                Step::None
            }
        };
        match self.read_during_write.take() {
            Some(read) => {
                let sequence = self.read_sequence;
                self.read(sequence, read)
            }
            None => step,
        }
    }
}

#[cfg(test)]
#[path = "editor_tests.rs"]
mod tests;
