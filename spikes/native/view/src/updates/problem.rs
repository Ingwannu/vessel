//! Recovery choices for update failures, never a raw diagnostic.

use crate::localization::Translator;

/// Safe recovery sentences shared by native update failure and restart paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// The update check could not be completed. Try again later.
    Check,
    /// The update information is too large for this app. Try again later.
    FeedTooLarge,
    /// The update information could not be read. Try again later.
    FeedInvalid,
    /// The update information could not be verified. This app was not changed.
    FeedUnverified,
    /// This app cannot read the newer update format. Install the latest build manually.
    FeedNewFormat,
    /// This update has no build for this device.
    NoBuild,
    /// The update could not be downloaded. Check your connection and try again.
    Download,
    /// The update could not be saved beside this app. Check the app folder's write permissions and try again.
    DownloadWrite,
    /// The download could not be verified. It was not installed. Try again.
    DownloadIntegrity,
    /// The update could not replace this app. Check the app folder's write permissions and try again.
    InstallWrite,
    /// This build cannot update itself. Install the latest build manually.
    Unsupported,
    /// The update was interrupted. Try again.
    Interrupted,
    /// The updated app could not be started. Vessel is still open; try restarting it.
    Restart,
}

impl Problem {
    /// Borrow the complete recovery sentence from the prepared catalog.
    #[must_use]
    pub fn words(self, catalog: &Translator) -> &str {
        let words = catalog.update();
        match self {
            Self::Check => words.problem_check(),
            Self::FeedTooLarge => words.problem_feed_too_large(),
            Self::FeedInvalid => words.problem_feed_invalid(),
            Self::FeedUnverified => words.problem_feed_unverified(),
            Self::FeedNewFormat => words.problem_feed_new_format(),
            Self::NoBuild => words.problem_no_build(),
            Self::Download => words.problem_download(),
            Self::DownloadWrite => words.problem_download_write(),
            Self::DownloadIntegrity => words.problem_download_integrity(),
            Self::InstallWrite => words.problem_install_write(),
            Self::Unsupported => words.problem_unsupported(),
            Self::Interrupted => words.problem_interrupted(),
            Self::Restart => words.problem_restart(),
        }
    }
}
