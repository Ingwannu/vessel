//! Existing disk reconciliation, kept separate from write obligation lifetime.
use super::{ConflictKind, EditorModel, EditorStatus, Read, Step};
use crate::workspace::text_format::TextFormat;
use vessel_proto::workspace::FileVersion;
impl EditorModel {
    /// A read's answer: the first read, a reload, or the panel reading the
    /// file again (a git write). A clean buffer follows the disk; a
    /// modified one conflicts unless the disk matches it.
    pub fn read(&mut self, sequence: u64, read: Read) -> Step {
        if sequence != self.read_sequence {
            return Step::None;
        }
        if !self.loaded() {
            return match read {
                Read::Text(text, version) => self.apply(&text, version),
                Read::Deleted | Read::Unverified | Read::Failed(_) => {
                    self.status = EditorStatus::Failed;
                    self.error = match read {
                        Read::Failed(sentence) => Some(sentence),
                        Read::Deleted | Read::Unverified | Read::Text(..) => None,
                    };
                    Step::None
                }
            };
        }
        // A disk observation cannot prove an earlier remote write terminated.
        // Do not replay such an observation when definitive completion arrives.
        if self.write_state() == Some(super::WriteState::Uncertain) {
            return Step::None;
        }
        if self.write_state() == Some(super::WriteState::Ended) {
            return self.read_ended(read);
        }
        if self.writing.is_some() {
            self.read_during_write = Some(read);
            return Step::None;
        }
        if std::mem::take(&mut self.reloading)
            && let Read::Text(text, version) = read
        {
            return self.apply(&text, version);
        }
        match read {
            Read::Deleted => self.enter_conflict(ConflictKind::Deleted),
            Read::Unverified => self.enter_conflict(ConflictKind::Unverified),
            // A reload that fails keeps the buffer (Paseo: a loaded editor
            // ignores a failed read).
            Read::Failed(_) => Step::None,
            Read::Text(text, version) => {
                let (format, normalized) = TextFormat::decode(&text);
                if normalized == self.persisted {
                    self.format = format;
                    self.version = Some(version);
                    if self.status == EditorStatus::Conflict {
                        self.conflict = None;
                        self.status = if self.modified() {
                            EditorStatus::Dirty
                        } else {
                            EditorStatus::Clean
                        };
                        if self.status == EditorStatus::Dirty {
                            return Step::Autosave;
                        }
                    }
                    Step::None
                } else if !self.modified() {
                    self.apply(&text, version)
                } else {
                    self.enter_conflict(ConflictKind::Changed)
                }
            }
        }
    }

    /// The read on a reload or after a conflict: the buffer is the disk's.
    pub(super) fn apply(&mut self, text: &str, version: FileVersion) -> Step {
        let (format, text) = TextFormat::decode(text);
        self.format = format;
        self.flush_after_recovery = super::FlushAfterRecovery::Automatic;
        self.write_sequence = self.write_sequence.saturating_add(1);
        self.writing = None;
        self.persisted.clone_from(&text);
        self.content = text;
        self.version = Some(version);
        self.status = EditorStatus::Clean;
        self.conflict = None;
        self.error = None;
        Step::StopAutosave
    }

    pub(super) fn enter_conflict(&mut self, kind: ConflictKind) -> Step {
        self.status = EditorStatus::Conflict;
        self.conflict = Some(kind);
        self.error = None;
        Step::StopAutosave
    }
}
