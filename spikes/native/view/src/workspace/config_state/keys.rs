//! Correlation is scoped to canonical project and connection, never a filesystem path.
use crate::connection::ConnectionContext;
use vessel_proto::{
    ProjectId,
    registry::{ConfigRevision, ConfigText, WriteProjectConfig},
};

/// Exact project-settings target; it carries no new node/transport identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigTarget {
    context: ConnectionContext,
    project: ProjectId,
}
impl ConfigTarget {
    /// Bind the registered id to the person's original authenticated admission.
    #[must_use]
    pub const fn new(context: ConnectionContext, project: ProjectId) -> Self {
        Self { context, project }
    }
    /// Original physical-admission value; callers must still validate its real weak guard.
    #[must_use]
    pub const fn context(&self) -> &ConnectionContext {
        &self.context
    }
    /// Registered project whose source is being edited.
    #[must_use]
    pub const fn project(&self) -> ProjectId {
        self.project
    }
}

/// One local read ticket, not a credential or file revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigReadKey {
    target: ConfigTarget,
    serial: u64,
}
impl ConfigReadKey {
    pub(super) const fn new(target: ConfigTarget, serial: u64) -> Self {
        Self { target, serial }
    }
    /// Captured target; never re-resolve an alias when settling a reply.
    #[must_use]
    pub const fn target(&self) -> &ConfigTarget {
        &self.target
    }
}

/// Immutable save intent uses node-returned revision and the exact submitted source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigSaveKey {
    target: ConfigTarget,
    serial: u64,
    expected: ConfigRevision,
    text: ConfigText,
}
impl ConfigSaveKey {
    pub(super) const fn new(
        target: ConfigTarget,
        serial: u64,
        expected: ConfigRevision,
        text: ConfigText,
    ) -> Self {
        Self {
            target,
            serial,
            expected,
            text,
        }
    }
    /// Original target admission.
    #[must_use]
    pub const fn target(&self) -> &ConfigTarget {
        &self.target
    }
    /// Exact precondition supplied by the authoritative read.
    #[must_use]
    pub const fn expected(&self) -> &ConfigRevision {
        &self.expected
    }
    /// Submitted source is private/redacted in Debug but explicit for dispatch.
    #[must_use]
    pub const fn text(&self) -> &ConfigText {
        &self.text
    }
    /// Existing canonical request value; no method registration is implied.
    #[must_use]
    pub fn request(&self) -> WriteProjectConfig {
        WriteProjectConfig {
            project: self.target.project,
            expected: self.expected.clone(),
            text: self.text.clone(),
        }
    }
}
