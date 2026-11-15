//! Updates in a browser: none. A page is updated by serving a new build
//! and reloading it; there is no installed binary to replace.
//!
//! [Decision Log]
//! - Purpose: the browser build (docs/plans/backend-m12-browser.md)
//!   compiles the same shell, title bar and About screen as the desktop,
//!   which show the updater's state (`update.rs`).
//! - Constraint: the updater (track R's `vessel-app-updater`) is built on
//!   reqwest with rustls, tokio's runtime and files, none of which build
//!   for wasm32; a dependency cannot point at another crate per target.
//! - Alternatives: a cfg at every place the shell, the title bar and About
//!   touch the updater (a dozen attributes in other tracks' files); a
//!   feature in the updater that leaves only its types (track R's crate).
//! - Choice: this module stands in for `update.rs` on wasm32 (`main.rs`
//!   picks it by path), with the same names: the state is always `Idle`,
//!   the build "does not update itself" (About says so), and the actions
//!   do nothing. `UpdateState` keeps the updater's variants so the screens'
//!   matches build unchanged; its fields are plain values here.
//! - Cost: the variants are kept in step with the updater's by hand; a new
//!   one there breaks the browser build until it is added here.

use std::fmt;
use std::sync::Arc;

use vessel_app_kit::ui::Context;

use crate::shell::Shell;

/// Never present in a browser.
#[derive(Debug)]
pub(crate) enum Updates {}

/// The updater's states (`vessel_app_updater::UpdateState`); a browser
/// build stays `Idle`.
#[derive(Debug, Clone)]
#[expect(
    dead_code,
    reason = "only `Idle` is built in a browser; the rest mirror the desktop's for the screens' matches"
)]
pub(crate) enum UpdateState {
    /// Nothing done yet.
    Idle,
    /// Asking the feed.
    Checking,
    /// The newest build runs.
    UpToDate {
        /// The feed's newest.
        latest: Words,
    },
    /// A build being fetched.
    Downloading {
        /// Which build.
        label: Words,
        /// Bytes so far.
        received: Count,
        /// Bytes in all.
        total: Count,
    },
    /// A build in place, waiting for a restart.
    Ready {
        /// Which build.
        label: Words,
        /// Its notes.
        notes: Option<Words>,
    },
    /// The last attempt failed.
    Failed {
        /// Why.
        error: Arc<Words>,
    },
}

/// A label, a note or an error's words, shown as they are.
#[derive(Debug, Clone)]
pub(crate) struct Words(String);

impl fmt::Display for Words {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A byte count, read with `get` as the updater's is.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Count(u64);

impl Count {
    pub(crate) const fn get(self) -> u64 {
        self.0
    }
}

/// The note shown once after an update (never, in a browser).
#[derive(Debug, Clone)]
pub(crate) struct Updated {
    pub(crate) label: String,
    pub(crate) notes: Option<String>,
}

/// Nothing to start.
pub(crate) const fn start(_shell: &mut Shell, _cx: &mut Context<'_, Shell>) {}

impl Shell {
    /// Nothing to check.
    pub(crate) const fn check_for_updates(&mut self, _cx: &mut Context<'_, Self>) {}

    /// Nothing to switch.
    pub(crate) const fn set_update_automatically(
        &mut self,
        _on: bool,
        _cx: &mut Context<'_, Self>,
    ) {
    }

    /// Nothing to restart into.
    pub(crate) const fn restart_to_update(&mut self, _cx: &mut Context<'_, Self>) {}

    /// A page does not update itself: there is never an updater.
    pub(crate) const fn updates_on(&self) -> bool {
        self.updates.runtime.is_some()
    }

    /// Nothing checks by itself.
    pub(crate) const fn updates_automatic(&self) -> bool {
        false
    }

    /// Nothing to announce.
    pub(crate) const fn announce_update(&mut self) {}
}
