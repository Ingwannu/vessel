//! Global UI availability follows the configured client book, independently of
//! viewport and connection authority. A configured offline host still counts.
use crate::{routes::Route, shell::Shell};

impl Shell {
    /// Whether a successful client-book snapshot has been installed at least
    /// once. Fleet refresh publishes only after saved_entries succeeds; offline
    /// entries remain in that snapshot. This grants no connection authority.
    pub(crate) const fn client_book_read(&self) -> bool {
        self.loaded
    }

    /// The book has been read and contains a host, or Settings is open. This is
    /// UI admission only; feature owners still admit their own resource effects.
    pub(crate) fn global_shortcuts_enabled(&self) -> bool {
        self.route == Route::Settings
            || (self.client_book_read() && !self.fleet.machines.is_empty())
    }
}

/// A book transition retires prefixes before a later read can re-enable input.
/// The existing Fleet event is the only book publisher; no cache is introduced.
pub(crate) fn book_changed(shell: &mut Shell) {
    if !shell.global_shortcuts_enabled() {
        shell.areas.shortcuts.engine.reset();
        shell.areas.shortcuts.held.release();
        super::badges::clear(&mut shell.areas.shortcuts);
        shell.areas.shortcuts.page.leave();
    }
}
