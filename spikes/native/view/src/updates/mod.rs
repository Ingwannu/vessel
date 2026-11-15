//! Update sentences prepared when state or language changes, never in a frame.
//!
//! [Decision Log]
//! - Purpose: title-bar and About messages share one prepared projection.
//! - Constraint: view cannot depend on the updater's native I/O/runtime crate.
//! - Choice: the host projects its typed state here; generated Fluent messages
//!   own whole sentences, and retained Arc values can enter GPUI without copies.
//! - Cost: the host must refresh this projection after an updater event or a
//!   language change. Its original typed updater state stays authoritative.

use std::sync::Arc;

use crate::localization::{
    FormatError, Number, Translator, UpdateStatusDownloadingKind, UpdateStatusReadyKind,
    UpdateUpdateToKind,
};

/// A build's display name; a numbered release gets the reference's `v` prefix.
/// This is presentation of an already accepted label, not release validation.
#[derive(Debug, Clone)]
pub struct BuildName {
    text: Arc<str>,
    kind: NameKind,
}

#[derive(Debug, Clone, Copy)]
enum NameKind {
    Numbered,
    Word,
}

impl From<&str> for BuildName {
    fn from(text: &str) -> Self {
        Self {
            text: Arc::from(text),
            kind: if text.starts_with(|c: char| c.is_ascii_digit()) {
                NameKind::Numbered
            } else {
                NameKind::Word
            },
        }
    }
}

impl BuildName {
    fn version(&self, catalog: &Translator) -> Result<Arc<str>, FormatError> {
        match self.kind {
            NameKind::Numbered => catalog.update().version(&self.text).map(Arc::from),
            NameKind::Word => Ok(Arc::clone(&self.text)),
        }
    }
    fn action(&self, catalog: &Translator) -> Result<Arc<str>, FormatError> {
        let kind = match self.kind {
            NameKind::Numbered => UpdateUpdateToKind::Numbered,
            NameKind::Word => UpdateUpdateToKind::Word,
        };
        catalog.update().update_to(kind, &self.text).map(Arc::from)
    }

    fn downloading(&self, catalog: &Translator) -> Result<Arc<str>, FormatError> {
        let kind = match self.kind {
            NameKind::Numbered => UpdateStatusDownloadingKind::Numbered,
            NameKind::Word => UpdateStatusDownloadingKind::Word,
        };
        catalog
            .update()
            .status_downloading(kind, &self.text)
            .map(Arc::from)
    }

    fn ready(&self, catalog: &Translator) -> Result<Arc<str>, FormatError> {
        let kind = match self.kind {
            NameKind::Numbered => UpdateStatusReadyKind::Numbered,
            NameKind::Word => UpdateStatusReadyKind::Word,
        };
        catalog
            .update()
            .status_ready(kind, &self.text)
            .map(Arc::from)
    }
}

/// The updater facts needed to choose its whole UI sentences. The host keeps
/// transport diagnostics and byte counts out of this presentation boundary.
#[derive(Debug, Clone)]
pub enum Status {
    /// No check yet.
    Idle,
    /// A check is running.
    Checking,
    /// The current build is newest.
    Current,
    /// A newer numbered build is being downloaded.
    Downloading {
        /// Its already validated release label, without a display prefix.
        build: BuildName,
        /// A numeric percentage, clamped by the host's byte-count adapter.
        progress: Number,
    },
    /// A numbered build is ready to install.
    Ready {
        /// Its already validated release label.
        build: BuildName,
    },
    /// The last attempt failed; the host retains its diagnostic separately.
    Failed,
}

/// The complete dynamic text shared by About and the title bar. Plain labels
/// stay in the catalog; these values are shared while a frame is drawn.
#[derive(Debug)]
pub struct PreparedUpdate {
    version: Arc<str>,
    status: Arc<str>,
    action: Arc<str>,
    progress: Option<Arc<str>>,
    banner: Option<Arc<str>>,
}

impl PreparedUpdate {
    /// Prepare all value messages together after a state or language change.
    /// # Errors
    /// Keeps the formatter's typed error if both translation and fallback fail.
    pub fn new(
        catalog: &Translator,
        running: &BuildName,
        status: &Status,
        announced: Option<&BuildName>,
    ) -> Result<Self, FormatError> {
        let words = catalog.update();
        let (status, action, progress) = match status {
            Status::Idle => (
                Arc::from(words.status_idle()),
                Arc::from(words.update()),
                None,
            ),
            Status::Checking => (
                Arc::from(words.status_checking()),
                Arc::from(words.update()),
                None,
            ),
            Status::Current => (
                Arc::from(words.status_up_to_date()),
                Arc::from(words.update()),
                None,
            ),
            Status::Failed => (
                Arc::from(words.status_failed()),
                Arc::from(words.update()),
                None,
            ),
            Status::Downloading { build, progress } => (
                build.downloading(catalog)?,
                Arc::from(words.update()),
                Some(Arc::from(words.downloading(progress.clone())?)),
            ),
            Status::Ready { build } => (build.ready(catalog)?, build.action(catalog)?, None),
        };
        Ok(Self {
            version: running.version(catalog)?,
            status,
            action,
            progress,
            banner: announced
                .map(|build| words.updated_to(&build.text).map(Arc::from))
                .transpose()?,
        })
    }

    /// The running build, including the reference's prefix for numbered labels.
    #[must_use]
    pub fn version(&self) -> Arc<str> {
        Arc::clone(&self.version)
    }

    /// About's whole current status sentence.
    #[must_use]
    pub fn status(&self) -> Arc<str> {
        Arc::clone(&self.status)
    }

    /// About's update action, naming a ready build only when one exists.
    #[must_use]
    pub fn action(&self) -> Arc<str> {
        Arc::clone(&self.action)
    }

    /// Title-bar download progress, present only during a download.
    #[must_use]
    pub fn progress(&self) -> Option<Arc<str>> {
        self.progress.as_ref().map(Arc::clone)
    }

    /// The whole updated-on-start notice, only while its typed announcement exists.
    #[must_use]
    pub fn banner(&self) -> Option<Arc<str>> {
        self.banner.as_ref().map(Arc::clone)
    }
}

#[cfg(test)]
mod tests;
