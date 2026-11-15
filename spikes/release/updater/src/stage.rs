//! Downloading an artifact next to the installed app, and proving it is
//! the one the manifest names before anything uses it.
//!
//! The file is written beside what it will replace, in the same
//! directory, so the swap is a rename on one volume (atomic, and the only
//! way to replace a running executable on Windows). It is hashed as it
//! streams, never held in memory whole (Tauri's and cargo-packager's
//! updaters read the artifact into a `Vec`; a GPUI app is tens of MiB),
//! and it is refused the moment it passes the size the manifest gives.
//! Its name starts with [`STAGING_PREFIX`], so a download cut short by
//! the app quitting is removed at the next start
//! ([`crate::install::Installation::clean_leftovers`]).

use std::path::{Path, PathBuf};

use sha2::Digest as _;
use tokio::io::AsyncWriteExt as _;

use crate::{
    feed::{FeedClient, FeedUrl, FetchError, READ_STALL},
    label::Label,
    manifest::{Artifact, ArtifactPath, ByteCount, Sha256},
};

/// One file the signed manifest names: the app's executable or one of
/// its companions (ADR 0094).
#[derive(Debug)]
pub(crate) struct Wanted<'a> {
    /// Where it is under the feed.
    pub(crate) path: &'a ArtifactPath,
    /// Its exact size.
    pub(crate) size: ByteCount,
    /// Its SHA-256.
    pub(crate) sha256: Sha256,
    /// A companion's role, in its staging file's name so the files of one
    /// set do not collide; `None` for the app, whose staging name stays
    /// what it was before the set.
    pub(crate) tag: Option<String>,
}

/// What every staging file's name starts with.
pub(crate) const STAGING_PREFIX: &str = ".vessel-update-";

/// An artifact on disk whose size and SHA-256 match the signed manifest.
#[derive(Debug)]
pub struct Staged {
    path: PathBuf,
    label: Label,
}

impl Staged {
    /// Where it is.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The build it is.
    #[must_use]
    pub fn label(&self) -> &Label {
        &self.label
    }

    /// A staged file the tests of the swap made themselves.
    #[cfg(test)]
    pub(crate) fn for_test(path: PathBuf, label: Label) -> Self {
        Self { path, label }
    }
}

/// Why a download did not produce a [`Staged`] file.
#[derive(Debug, thiserror::Error)]
pub enum StageError {
    /// The request failed.
    #[error("downloading the update failed")]
    Fetch(#[source] FetchError),
    /// The staging file could not be written beside the app (a read-only
    /// location, such as a system-wide install without rights to it).
    #[error("the update could not be written beside the app")]
    Write(#[source] std::io::Error),
    /// More bytes arrived than the manifest's size.
    #[error("the download is larger than the signed manifest says")]
    TooLarge,
    /// The body ended before the manifest's size.
    #[error("the download ended early ({received} of {expected} bytes)")]
    Short {
        /// Bytes received.
        received: u64,
        /// Bytes the manifest gives.
        expected: u64,
    },
    /// The bytes are not the ones the manifest signed for.
    #[error("the download's SHA-256 is not the signed manifest's")]
    Digest,
}

/// Download `artifact` of build `label` into `dir`, calling `progress`
/// with the bytes received so far, at most 64 times.
///
/// # Errors
/// [`StageError`] for a failed request, a write that fails, and a body
/// that is not exactly the manifest's bytes; the partial file is removed.
pub async fn download(
    client: &FeedClient,
    feed: &FeedUrl,
    artifact: &Artifact,
    label: &Label,
    dir: &Path,
    progress: impl FnMut(ByteCount),
) -> Result<Staged, StageError> {
    let wanted = Wanted {
        path: &artifact.path,
        size: artifact.size,
        sha256: artifact.sha256,
        tag: None,
    };
    download_file(client, feed, &wanted, label, dir, progress).await
}

/// [`download`] for any file of a set.
///
/// # Errors
/// As [`download`].
pub(crate) async fn download_file(
    client: &FeedClient,
    feed: &FeedUrl,
    wanted: &Wanted<'_>,
    label: &Label,
    dir: &Path,
    progress: impl FnMut(ByteCount),
) -> Result<Staged, StageError> {
    let pid = std::process::id();
    let name = match &wanted.tag {
        None => format!("{STAGING_PREFIX}{label}.{pid}.part"),
        Some(tag) => format!("{STAGING_PREFIX}{label}.{pid}.{tag}.part"),
    };
    let path = dir.join(name);
    let written = write(client, feed, wanted, &path, progress).await;
    match written {
        Ok(()) => Ok(Staged {
            path,
            label: label.clone(),
        }),
        Err(error) => {
            // The file is ours (created new above), so removing it cannot
            // touch anything else; a failure to remove leaves it for the
            // next start's clean-up.
            if let Err(remove) = tokio::fs::remove_file(&path).await {
                tracing::warn!(
                    error = %crate::report::chain(&remove),
                    "a partial update was left for the next start"
                );
            }
            Err(error)
        }
    }
}

async fn write(
    client: &FeedClient,
    feed: &FeedUrl,
    artifact: &Wanted<'_>,
    path: &Path,
    mut progress: impl FnMut(ByteCount),
) -> Result<(), StageError> {
    let expected = artifact.size.get();
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .await
        .map_err(StageError::Write)?;
    tracing::debug!(
        staged = %crate::service::file_name(path),
        "the staging file is created beside the app"
    );
    let mut response = client
        .artifact(feed, artifact.path)
        .await
        .map_err(StageError::Fetch)?;
    tracing::info!(
        status = %response.status(),
        length = ?response.content_length(),
        "the feed answers the download"
    );
    let mut hasher = sha2::Sha256::new();
    let mut received: u64 = 0;
    // A report every 1/64 of the file: enough for a progress bar, and a
    // bound on how often the state is published.
    let step = expected.checked_div(64).unwrap_or(0).max(1);
    let mut next_report = step;
    loop {
        // How far it got goes to the log: a download that breaks off at
        // the same byte each time points at something on the way.
        let chunk = match tokio::time::timeout(READ_STALL, response.chunk()).await {
            Ok(Ok(chunk)) => chunk,
            Ok(Err(error)) => {
                tracing::warn!(received, expected, "the download broke off");
                return Err(StageError::Fetch(FetchError::unreachable(error)));
            }
            Err(_elapsed) => {
                tracing::warn!(received, expected, stalled_for = ?READ_STALL, "the download stalled");
                return Err(StageError::Fetch(FetchError::Stalled));
            }
        };
        let Some(chunk) = chunk else { break };
        let len = u64::try_from(chunk.len()).map_err(|_huge| StageError::TooLarge)?;
        received = received.checked_add(len).ok_or(StageError::TooLarge)?;
        if received > expected {
            return Err(StageError::TooLarge);
        }
        hasher.update(&chunk);
        file.write_all(&chunk).await.map_err(StageError::Write)?;
        if received >= next_report {
            next_report = received.saturating_add(step);
            if let Ok(count) = ByteCount::try_from(received) {
                progress(count);
            }
        }
    }
    if received != expected {
        return Err(StageError::Short { received, expected });
    }
    if Sha256::from_bytes(hasher.finalize().into()) != artifact.sha256 {
        return Err(StageError::Digest);
    }
    // Once, before the file replaces the app: a crash after the swap must
    // not leave a truncated executable in the app's place.
    file.sync_all().await.map_err(StageError::Write)?;
    Ok(())
}
