//! Revision-bound project source editing, without a second TOML parser or root identity.
//!
//! [Decision Log]
//! - Purpose: external edits and late saves cannot silently replace a newer draft.
//! - Constraint: only the node validates vessel.toml and returns content revisions.
//! - Alternatives: infer revisions locally, overwrite on conflict, or copy the schema.
//! - Choice: preserve bounded source, original connection, project and operation key;
//!   stale/uncertain saves require an explicit reload and successful replies must
//!   match the submitted source. Editing during save preserves the later draft.
//! - Impact: this pure model does not validate a transport guard or authorize I/O.
//!   The actual caller must check its original weak guard at installation/query.

mod keys;
pub use keys::{ConfigReadKey, ConfigSaveKey, ConfigTarget};
use vessel_proto::registry::{
    ConfigDocument, ConfigRevision, ConfigText, ConfigTextError, ConfigValidity,
};

/// Typed read/save failure classification; no source or peer diagnostic is retained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigProblem {
    /// Current source differs from the explicit save precondition.
    Conflict,
    /// The write may have reached the node and requires an authoritative reload.
    Uncertain,
    /// The node rejected source/schema validation; the draft stays editable.
    Invalid,
    /// Permission or project authority refused this operation.
    Refused,
    /// The exact original peer cannot provide configuration editing.
    Unsupported,
    /// The original connection is no longer available.
    Offline,
    /// A reply's source/revision/validity does not match this save.
    Incoherent,
}

/// Reloading a dirty draft is a deliberate UI action, never a side effect of refresh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadDraft {
    /// Refuse reload if it would discard unsaved text.
    Preserve,
    /// The person explicitly requested discarding and reloading source.
    Discard,
}

/// Local operation refusals do not claim that a remote effect was rolled back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ConfigRefusal {
    /// One operation in this editor is still outstanding.
    #[error("project configuration is busy")]
    Busy,
    /// Explicit reload/discard is required to replace this draft.
    #[error("project configuration has unsaved changes")]
    Dirty,
    /// No authoritative current document has been read.
    #[error("project configuration must be read first")]
    NotHydrated,
    /// Conflict or uncertainty must be reconciled before another save.
    #[error("project configuration must be reloaded")]
    ReloadRequired,
    /// The canonical text budget rejected this edit.
    #[error(transparent)]
    Text(#[from] ConfigTextError),
    /// The draft matches the authoritative source.
    #[error("project configuration has no changes")]
    Unchanged,
    /// A late reply must never reuse an exhausted operation identity.
    #[error("project configuration operation identifiers are exhausted")]
    Exhausted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Pending {
    Read(ConfigReadKey),
    Save(ConfigSaveKey),
}

/// One active editor, with bounded canonical text and monotonically unique tickets.
#[derive(Debug, Default)]
pub struct ProjectConfig {
    target: Option<ConfigTarget>,
    serial: u64,
    pending: Option<Pending>,
    document: Option<ConfigDocument>,
    draft: Option<ConfigText>,
    problem: Option<ConfigProblem>,
    reload_required: bool,
}
impl ProjectConfig {
    /// Start a read on the exact original admission and registered project.
    ///
    /// # Errors
    /// Outstanding operation, dirty draft without discard, or exhausted ticket.
    pub fn begin_read(
        &mut self,
        target: ConfigTarget,
        reload: ReloadDraft,
    ) -> Result<ConfigReadKey, ConfigRefusal> {
        if self.pending.is_some() {
            return Err(ConfigRefusal::Busy);
        }
        if self.dirty() && reload == ReloadDraft::Preserve {
            return Err(ConfigRefusal::Dirty);
        }
        let serial = self.next_serial()?;
        let key = ConfigReadKey::new(target.clone(), serial);
        self.target = Some(target);
        self.pending = Some(Pending::Read(key.clone()));
        self.document = None;
        self.draft = None;
        self.problem = None;
        self.reload_required = false;
        Ok(key)
    }

    /// Only the currently outstanding read can replace the editor's source.
    /// The caller first validates the real original delivery guard.
    pub fn read(
        &mut self,
        key: &ConfigReadKey,
        result: Result<ConfigDocument, ConfigProblem>,
    ) -> bool {
        if self.pending.as_ref() != Some(&Pending::Read(key.clone()))
            || self.target.as_ref() != Some(key.target())
        {
            return false;
        }
        self.pending = None;
        match result {
            Ok(document)
                if matches!(document.revision, ConfigRevision::Missing)
                    && (!document.text.as_str().is_empty()
                        || document.validity != ConfigValidity::Valid) =>
            {
                self.problem = Some(ConfigProblem::Incoherent);
            }
            Ok(document) => {
                self.draft = Some(document.text.clone());
                self.document = Some(document);
                self.problem = None;
            }
            Err(problem) => self.problem = Some(problem),
        }
        true
    }

    /// Retain an edit only within the canonical byte/NUL budget; TOML validation
    /// remains on the node. Saving never silently normalizes comments or unknown keys.
    ///
    /// # Errors
    /// No document is loaded, or the canonical text budget/NUL rule rejected this edit.
    pub fn edit(&mut self, text: &str) -> Result<(), ConfigRefusal> {
        if self.document.is_none() {
            return Err(ConfigRefusal::NotHydrated);
        }
        let text = ConfigText::parse(text)?;
        self.draft = Some(text);
        Ok(())
    }

    /// Capture the authoritative revision and exact text before dispatch.
    ///
    /// # Errors
    /// Busy, unhydrated, unchanged, conflicted/uncertain or exhausted state.
    pub fn begin_save(&mut self) -> Result<ConfigSaveKey, ConfigRefusal> {
        if self.pending.is_some() {
            return Err(ConfigRefusal::Busy);
        }
        if self.reload_required {
            return Err(ConfigRefusal::ReloadRequired);
        }
        let target = self.target.clone().ok_or(ConfigRefusal::NotHydrated)?;
        let document = self.document.as_ref().ok_or(ConfigRefusal::NotHydrated)?;
        let text = self.draft.clone().ok_or(ConfigRefusal::NotHydrated)?;
        if text == document.text {
            return Err(ConfigRefusal::Unchanged);
        }
        let expected = document.revision.clone();
        let serial = self.next_serial()?;
        let key = ConfigSaveKey::new(target, serial, expected, text);
        self.pending = Some(Pending::Save(key.clone()));
        self.problem = None;
        Ok(key)
    }

    /// A successful save advances the base document while preserving edits made
    /// after dispatch. A stale/uncertain save can only be reconciled by reload.
    pub fn saved(
        &mut self,
        key: &ConfigSaveKey,
        result: Result<ConfigDocument, ConfigProblem>,
    ) -> bool {
        if self.pending.as_ref() != Some(&Pending::Save(key.clone()))
            || self.target.as_ref() != Some(key.target())
        {
            return false;
        }
        self.pending = None;
        let result = result.and_then(|document| {
            if document.text != *key.text()
                || document.validity != ConfigValidity::Valid
                || !matches!(document.revision, ConfigRevision::Present { .. })
                || document.revision == *key.expected()
            {
                Err(ConfigProblem::Incoherent)
            } else {
                Ok(document)
            }
        });
        match result {
            Ok(document) => {
                self.document = Some(document);
                self.problem = None;
            }
            Err(problem) => {
                self.reload_required = matches!(
                    problem,
                    ConfigProblem::Conflict
                        | ConfigProblem::Uncertain
                        | ConfigProblem::Offline
                        | ConfigProblem::Incoherent
                );
                self.problem = Some(problem);
            }
        }
        true
    }

    /// Stop local observation without pretending to cancel an already sent write.
    /// A later open uses a fresh monotonic ticket, even for the same target.
    pub fn detach(&mut self) {
        self.target = None;
        self.pending = None;
        self.document = None;
        self.draft = None;
        self.problem = None;
        self.reload_required = false;
    }

    fn next_serial(&mut self) -> Result<u64, ConfigRefusal> {
        let serial = self.serial.checked_add(1).ok_or(ConfigRefusal::Exhausted)?;
        self.serial = serial;
        Ok(serial)
    }
    /// Whether text differs from the current authoritative source.
    #[must_use]
    pub fn dirty(&self) -> bool {
        self.document
            .as_ref()
            .zip(self.draft.as_ref())
            .is_some_and(|(document, draft)| document.text != *draft)
    }
    /// Authoritative revision, source and validity; absent does not imply a missing file.
    #[must_use]
    pub const fn document(&self) -> Option<&ConfigDocument> {
        self.document.as_ref()
    }
    /// The bounded draft explicitly requested for display, never debug/log interpolation.
    #[must_use]
    pub const fn draft(&self) -> Option<&ConfigText> {
        self.draft.as_ref()
    }
    /// Current structured refusal, without peer or configuration bytes.
    #[must_use]
    pub const fn problem(&self) -> Option<ConfigProblem> {
        self.problem
    }
    /// One read or save still needs a correlated settlement.
    #[must_use]
    pub const fn pending(&self) -> bool {
        self.pending.is_some()
    }
    /// Save is fenced until the person explicitly reloads authoritative source.
    #[must_use]
    pub const fn reload_required(&self) -> bool {
        self.reload_required
    }
}

#[cfg(test)]
mod tests;
