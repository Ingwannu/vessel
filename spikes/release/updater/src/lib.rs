//! How Vessel's native app updates itself (milestone 12, question 8 of
//! `docs/plans/backend-m12.md`; the design is in
//! `docs/plans/backend-m12-release.md`).
//!
//! A release is published as a **feed**: a directory any static HTTPS
//! host can serve, holding one signed manifest per channel
//! (`<channel>.json`) and the artifacts it names. A copy of the app
//! checks the feed soon after start and every hour, and when the
//! channel's build is newer than its own it downloads the artifact for
//! its system and packaging beside itself, checks it against the signed
//! size and SHA-256, puts it in its own place, and tells the owner to
//! restart. Nothing here knows of the UI layer: the app reads
//! [`UpdateState`] and calls [`Updater::restart`].
//!
//! Zed's and Tauri 2's updaters were read for this, as references only
//! (Zed's is GPL; Tauri's is MIT or Apache-2.0 but was not taken as code
//! either; the plan says why). Taken from them as design: Zed's hourly
//! check, background download and "restart to update", and its keeping
//! the restart in the app's hands; Tauri's minisign signature, its feed
//! keyed by system, its build-time public key. Not taken: Zed's unsigned
//! downloads, Tauri's unsigned manifest, both running an installer to
//! update on Windows, cargo-packager's whole-file download into memory
//! and its exiting the process from inside the library.
//!
//! What the app writes to use it:
//!
//! ```no_run
//! use vessel_app_updater::{Installation, Target, UpdateState, Updater, settings_from_build};
//!
//! # fn app(runtime: &tokio::runtime::Handle) -> Result<(), Box<dyn std::error::Error>> {
//! // At start, once: a development build has no settings and no updater.
//! let updater = match settings_from_build()? {
//!     Some(settings) => {
//!         let installation = Installation::from_process(settings.target.default_packaging())?;
//!         Some(Updater::start(settings, installation, runtime)?)
//!     }
//!     None => None,
//! };
//! // In the UI: show the state, wake on `changed()`.
//! if let Some(updater) = &updater {
//!     if let UpdateState::Ready { label, .. } = &*updater.state().borrow() {
//!         // "Restart to update to {label}"; when clicked:
//!         updater.restart(std::env::args_os().skip(1))?;
//!         // ...and then quit the app (GPUI: `cx.quit()`).
//!     }
//! }
//! # let _ = Target::current();
//! # Ok(())
//! # }
//! ```

mod channel;
mod config;
mod feed;
mod install;
mod key;
mod label;
mod manifest;
mod memory;
mod platform;
#[cfg(windows)]
mod registry;
mod report;
mod restart;
mod service;
mod stage;

pub use channel::{Channel, ChannelError};
pub use config::{SettingsError, build_label, settings_from_build};
pub use feed::{FeedClient, FeedUrl, FeedUrlError, FetchError};
pub use install::{DetectError, InstallError, Installation};
pub use key::{KeyError, PublicKey};
pub use label::{Label, LabelError};
pub use manifest::{
    Artifact, ArtifactPath, ArtifactPathError, ByteCount, ByteCountError, COMPANION_DIR, Companion,
    Compressed, Encoding, Envelope, MAX_ARTIFACT_BYTES, MAX_ENVELOPE_BYTES, Manifest, Notes,
    OpenError, Published, Role, RoleError, SCHEMA, Sha256, Sha256Error, TooLong,
};
pub use memory::Memory;
pub use platform::{Packaging, Target, UnknownPackaging, UnknownTarget};
pub use report::chain;
pub use restart::{RESTARTED_FROM, RestartError, relaunch};
pub use service::{
    ApplyError, EVERY, FIRST_AFTER, Kept, RETRY_AFTER, Settings, UpdateError, UpdateState, Updater,
};
pub use stage::{StageError, Staged, download};
