//! Update sentences prepared when state or language changes, never in a frame.
//!
//! [Decision Log]
//! - Purpose: title-bar and About messages share one prepared projection.
//! - Constraint: view cannot depend on the updater's native I/O/runtime crate.
//! - Choice: the host projects its typed state here; generated Fluent messages
//!   own whole sentences, and retained Arc values can enter GPUI without copies.
//! - Cost: the host must refresh this projection after an updater event or a
//!   language change. Its original typed updater state stays authoritative.

mod problem;

pub use problem::Problem;

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
    Failed {
        /// The safe recovery, classified by the host without its diagnostic.
        problem: Problem,
    },
}

/// The complete dynamic text shared by About and the title bar. Plain labels
/// stay in the catalog; these values are shared while a frame is drawn.
#[derive(Debug)]
pub struct PreparedUpdate {
    version: Arc<str>,
    state: StatusText,
    banner: Option<Arc<str>>,
}

#[derive(Debug)]
struct StatusText {
    status: Arc<str>,
    action: Arc<str>,
    progress: Option<Arc<str>>,
    problem: Option<Arc<str>>,
}

impl StatusText {
    fn plain(catalog: &Translator, status: Arc<str>) -> Self {
        Self {
            status,
            action: Arc::from(catalog.update().update()),
            progress: None,
            problem: None,
        }
    }

    fn prepare(catalog: &Translator, status: &Status) -> Result<Self, FormatError> {
        let words = catalog.update();
        Ok(match status {
            Status::Idle => Self::plain(catalog, Arc::from(words.status_idle())),
            Status::Checking => Self::plain(catalog, Arc::from(words.status_checking())),
            Status::Current => Self::plain(catalog, Arc::from(words.status_up_to_date())),
            Status::Failed { problem } => Self {
                problem: Some(Arc::from(problem.words(catalog))),
                ..Self::plain(catalog, Arc::from(words.status_failed()))
            },
            Status::Downloading { build, progress } => Self {
                progress: Some(Arc::from(words.downloading(progress.clone())?)),
                ..Self::plain(catalog, build.downloading(catalog)?)
            },
            Status::Ready { build } => Self {
                status: build.ready(catalog)?,
                action: build.action(catalog)?,
                progress: None,
                problem: None,
            },
        })
    }
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
        Ok(Self {
            version: running.version(catalog)?,
            state: StatusText::prepare(catalog, status)?,
            banner: announced
                .map(|build| catalog.update().updated_to(&build.text).map(Arc::from))
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
        Arc::clone(&self.state.status)
    }

    /// About's update action, naming a ready build only when one exists.
    #[must_use]
    pub fn action(&self) -> Arc<str> {
        Arc::clone(&self.state.action)
    }

    /// Title-bar download progress, present only during a download.
    #[must_use]
    pub fn progress(&self) -> Option<Arc<str>> {
        self.state.progress.as_ref().map(Arc::clone)
    }

    /// The safe whole recovery sentence, only for a failed update.
    #[must_use]
    pub fn problem(&self) -> Option<Arc<str>> {
        self.state.problem.as_ref().map(Arc::clone)
    }

    /// The whole updated-on-start notice, only while its typed announcement exists.
    #[must_use]
    pub fn banner(&self) -> Option<Arc<str>> {
        self.banner.as_ref().map(Arc::clone)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod problem_tests;
