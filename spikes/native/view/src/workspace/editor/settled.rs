//! Reconcile only an attempt whose original handler is already known to have
//! ended. A read never releases the separate no-terminal-evidence state.
use super::{ConflictKind, EditorModel, EditorStatus, Read, Step, WriteState};
use crate::workspace::text_format::TextFormat;

impl EditorModel {
    pub(super) fn read_ended(&mut self, read: Read) -> Step {
        let Some(attempt) = self
            .writing
            .take_if(|attempt| attempt.state == WriteState::Ended)
        else {
            return Step::None;
        };
        match read {
            Read::Failed(_) => {
                // Keep the original failure and snapshot. Only a successful
                // subsequent acquisition can reconcile this known-ended call.
                self.writing = Some(attempt);
                Step::None
            }
            Read::Deleted => self.enter_conflict(ConflictKind::Deleted),
            Read::Unverified => self.enter_conflict(ConflictKind::Unverified),
            Read::Text(text, version) => {
                let (format, normalized) = TextFormat::decode(&text);
                if normalized == attempt.text {
                    self.persisted = normalized;
                    self.version = Some(version);
                    self.format = format;
                    self.error = None;
                    self.status = if self.modified() {
                        EditorStatus::Dirty
                    } else {
                        EditorStatus::Clean
                    };
                    // Recovery confirms a baseline, not permission to replay
                    // later edits under another admission.
                    Step::StopAutosave
                } else if normalized == self.persisted {
                    self.version = Some(version);
                    self.format = format;
                    self.status = EditorStatus::Error;
                    // Persistent rejection must not become an autosave loop.
                    Step::StopAutosave
                } else {
                    self.enter_conflict(ConflictKind::Changed)
                }
            }
        }
    }
}
