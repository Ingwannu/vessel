//! Downloading a file of the set next to the installed app, and proving it
//! is the one the manifest names before anything uses it.
//!
//! The file is written beside what it will replace, in the same
//! directory, so the swap is a rename on one volume (atomic, and the only
//! way to replace a running executable on Windows). It is hashed as it
//! streams, never held in memory whole (Tauri's and cargo-packager's
//! updaters read the artifact into a `Vec`; a GPUI app is tens of MiB),
//! and it is refused the moment it passes the size the manifest gives.
//!
//! [Decision Log] Resuming, and the compressed file (2026-11-15)
//! - Purpose: the owner's Windows copy took 0.0.0.0.19 over a slow line
//!   (about 0.27 MB/s). The download broke off at 31.9 of 51.7 MB, and
//!   every retry began again at zero. A download that breaks off must go
//!   on from where it stopped, and must carry fewer bytes to begin with.
//! - Constraints: the feed is static files behind a host the owner may
//!   change; a range is an HTTP feature the host may lack (the preview
//!   proxy answers one range since 2026-11-15). Every byte must still be
//!   the signed one. The app may quit at any moment.
//! - Alternatives: keep a download in memory and retry within one run
//!   (lost when the app quits, and the app is often closed on a slow
//!   line); chunked files with a list of hashes (a new feed layout every
//!   host must serve); a delta against the running build (a differ on both
//!   sides, for a gain only once the line is fast enough anyway).
//! - Choice: the staging file's name comes from the label and the digest
//!   of the bytes it will hold (not the process id), so a later attempt,
//!   in this run or the next, finds what an earlier one kept. The earlier
//!   bytes are hashed again, and the rest is asked for with
//!   `Range: bytes=<kept>-`. A server that answers the whole file (200)
//!   starts the file over; one that answers 416, or another range, is
//!   asked for the whole file. A download that broke off keeps its file;
//!   one whose bytes are wrong (size or digest) loses it. When the
//!   manifest offers a gzip copy, that copy is what travels: its own
//!   digest is checked before it is unpacked, and the unpacked file
//!   against the file's digest. The file being written is locked, so two
//!   copies of the app never write one file.
//! - Why: the feed stays plain files; old apps see nothing new (they send
//!   no range and read no `compressed`); a slow line gains twice, once
//!   from the smaller file and again from never paying for a byte twice.
//! - Cost: up to one set's files stay beside the app between attempts
//!   (the next manifest read removes the ones it no longer wants,
//!   [`crate::install::Installation::prune_staging`]); a resumed file is
//!   read once more to hash what was kept; unpacking needs the
//!   compressed file and the plain one on disk at once.

use std::{
    io::{Read as _, Write as _},
    path::{Path, PathBuf},
};

use sha2::Digest as _;
use tokio::io::{AsyncReadExt as _, AsyncSeekExt as _, AsyncWriteExt as _};

use crate::{
    feed::{FeedClient, FeedUrl, FetchError, READ_STALL},
    label::Label,
    manifest::{Artifact, ArtifactPath, ByteCount, Compressed, Encoding, Sha256},
};

/// What every staging file's name starts with.
pub(crate) const STAGING_PREFIX: &str = ".vessel-update-";

/// The pieces read from disk and from the network.
const CHUNK: usize = 64 * 1024;

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
    /// A companion's role, in its staging files' names so the files of
    /// one set do not collide; `None` for the app.
    pub(crate) tag: Option<String>,
    /// The same file compressed, when the manifest offers one.
    pub(crate) compressed: Option<&'a Compressed>,
}

impl Wanted<'_> {
    /// The compressed copy this build can unpack, if any.
    fn gzip(&self) -> Option<&Compressed> {
        self.compressed
            .filter(|compressed| compressed.encoding == Encoding::Gzip)
    }

    /// The bytes that travel: the compressed copy's when there is one.
    pub(crate) fn transfer_size(&self) -> u64 {
        self.gzip()
            .map_or(self.size.get(), |compressed| compressed.size.get())
    }

    /// The staging files of this file for `label`: the plain one, and the
    /// compressed one when it travels.
    pub(crate) fn staging_names(&self, label: &Label) -> Vec<String> {
        let mut names = vec![self.plain_name(label)];
        if let Some(compressed) = self.gzip() {
            names.push(self.packed_name(label, compressed));
        }
        names
    }

    fn plain_name(&self, label: &Label) -> String {
        format!(
            "{STAGING_PREFIX}{label}.{}{}.part",
            self.tag_dot(),
            short(&self.sha256)
        )
    }

    fn packed_name(&self, label: &Label, compressed: &Compressed) -> String {
        format!(
            "{STAGING_PREFIX}{label}.{}{}.gz.part",
            self.tag_dot(),
            short(&compressed.sha256)
        )
    }

    fn tag_dot(&self) -> String {
        self.tag
            .as_ref()
            .map_or_else(String::new, |tag| format!("{tag}."))
    }
}

/// The first 16 hexadecimal digits of a digest: enough to tell two builds'
/// files apart in a name.
fn short(digest: &Sha256) -> String {
    digest.to_string().chars().take(16).collect()
}

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
    /// The request failed or the body stopped. What arrived is kept, and
    /// the next attempt goes on from there.
    #[error("downloading the update failed")]
    Fetch(#[source] FetchError),
    /// The staging file could not be written beside the app (a read-only
    /// location, such as a system-wide install without rights to it).
    #[error("the update could not be written beside the app")]
    Write(#[source] std::io::Error),
    /// Another copy of the app is writing the same file.
    #[error("another copy of the app is downloading the update")]
    Busy,
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
    /// The compressed copy, though signed, did not unpack.
    #[error("the downloaded update did not unpack")]
    Unpack(#[source] std::io::Error),
}

impl StageError {
    /// Whether what arrived is worth keeping for the next attempt: only
    /// when the line, not the bytes, failed.
    const fn keeps_partial(&self) -> bool {
        matches!(self, Self::Fetch(_) | Self::Busy)
    }
}

/// Download `artifact` of build `label` into `dir`, calling `progress`
/// with the bytes received so far (counting those an earlier attempt
/// kept), at most about 64 times.
///
/// # Errors
/// [`StageError`] for a failed request, a write that fails, and a body
/// that is not exactly the manifest's bytes.
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
        compressed: artifact.compressed.as_ref(),
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
    let target = dir.join(wanted.plain_name(label));
    match wanted.gzip() {
        None => {
            let file = File {
                path: wanted.path,
                size: wanted.size,
                sha256: wanted.sha256,
            };
            fetch(client, feed, &file, &target, progress).await?;
        }
        Some(compressed) => {
            // The plain file may already be whole (unpacked by an attempt
            // that failed later, at the swap); the compressed one is not
            // needed then.
            if !whole(&target, wanted.size, wanted.sha256).await {
                let packed = dir.join(wanted.packed_name(label, compressed));
                let file = File {
                    path: &compressed.path,
                    size: compressed.size,
                    sha256: compressed.sha256,
                };
                fetch(client, feed, &file, &packed, progress).await?;
                let (size, sha256) = (wanted.size, wanted.sha256);
                let unpack_target = target.clone();
                let unpack_from = packed.clone();
                let unpacked = tokio::task::spawn_blocking(move || {
                    unpack(&unpack_from, &unpack_target, size, sha256)
                })
                .await
                .unwrap_or_else(|_cancelled| {
                    Err(StageError::Write(std::io::Error::other(
                        "unpacking was interrupted",
                    )))
                });
                // Verified and unpacked, or not unpackable: either way the
                // compressed copy has served.
                remove_quietly(&packed).await;
                if let Err(error) = unpacked {
                    remove_quietly(&target).await;
                    return Err(error);
                }
                tracing::info!(
                    staged = %crate::service::file_name(&target),
                    "the compressed update is unpacked and verified"
                );
            }
        }
    }
    Ok(Staged {
        path: target,
        label: label.clone(),
    })
}

/// One file that travels: the plain file or its compressed copy.
struct File<'a> {
    path: &'a ArtifactPath,
    size: ByteCount,
    sha256: Sha256,
}

/// Whether `path` already holds exactly `size` bytes of digest `sha256`.
async fn whole(path: &Path, size: ByteCount, sha256: Sha256) -> bool {
    let Ok(meta) = tokio::fs::metadata(path).await else {
        return false;
    };
    if meta.len() != size.get() {
        return false;
    }
    let Ok(mut file) = tokio::fs::File::open(path).await else {
        return false;
    };
    let mut hasher = sha2::Sha256::new();
    let mut buffer = vec![0_u8; CHUNK];
    loop {
        match file.read(&mut buffer).await {
            Ok(0) => break,
            Ok(read) => hasher.update(buffer.get(..read).unwrap_or_default()),
            Err(_error) => return false,
        }
    }
    Sha256::from_bytes(hasher.finalize().into()) == sha256
}

/// Remove a file that is no longer needed; a failure leaves it for the
/// next manifest read's pruning.
async fn remove_quietly(path: &Path) {
    match tokio::fs::remove_file(path).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => tracing::warn!(
            error = %crate::report::chain(&error),
            file = %crate::service::file_name(path),
            "a staging file was left for later"
        ),
    }
}

/// Bring `target` to exactly `file`'s bytes, going on from what it holds.
/// What arrived stays when the line fails; wrong bytes are removed.
async fn fetch(
    client: &FeedClient,
    feed: &FeedUrl,
    file: &File<'_>,
    target: &Path,
    progress: impl FnMut(ByteCount),
) -> Result<(), StageError> {
    let result = write(client, feed, file, target, progress).await;
    if let Err(error) = &result
        && !error.keeps_partial()
    {
        remove_quietly(target).await;
    }
    result
}

async fn write(
    client: &FeedClient,
    feed: &FeedUrl,
    file: &File<'_>,
    target: &Path,
    mut progress: impl FnMut(ByteCount),
) -> Result<(), StageError> {
    let expected = file.size.get();
    let opened = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(target)
        .map_err(StageError::Write)?;
    // Held until the file is dropped at the end of this function.
    match opened.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => return Err(StageError::Busy),
        Err(std::fs::TryLockError::Error(error)) => return Err(StageError::Write(error)),
    }
    let mut out = tokio::fs::File::from_std(opened);
    let mut hasher = sha2::Sha256::new();
    let mut have = kept(&mut out, &mut hasher, expected).await?;
    if have == expected {
        if Sha256::from_bytes(hasher.clone().finalize().into()) == file.sha256 {
            tracing::info!(
                staged = %crate::service::file_name(target),
                "an earlier attempt already holds the whole file"
            );
            out.sync_all().await.map_err(StageError::Write)?;
            return Ok(());
        }
        // Whole but not these bytes: start over.
        out.set_len(0).await.map_err(StageError::Write)?;
        hasher = sha2::Sha256::new();
        have = 0;
    }
    let (mut response, have) = rest(client, feed, file, &mut out, &mut hasher, have).await?;
    let mut received = have;
    if let Ok(count) = ByteCount::try_from(received) {
        progress(count);
    }
    // A report every 1/64 of the file: enough for a progress bar, and a
    // bound on how often the state is published.
    let step = expected.checked_div(64).unwrap_or(0).max(1);
    let mut next_report = received.saturating_add(step);
    loop {
        // How far it got goes to the log: a download that breaks off at
        // the same byte each time points at something on the way.
        let chunk = match tokio::time::timeout(READ_STALL, response.chunk()).await {
            Ok(Ok(chunk)) => chunk,
            Ok(Err(error)) => {
                tracing::warn!(
                    received,
                    expected,
                    "the download broke off; what arrived is kept"
                );
                out.flush().await.map_err(StageError::Write)?;
                return Err(StageError::Fetch(FetchError::unreachable(error)));
            }
            Err(_elapsed) => {
                tracing::warn!(received, expected, stalled_for = ?READ_STALL, "the download stalled; what arrived is kept");
                out.flush().await.map_err(StageError::Write)?;
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
        out.write_all(&chunk).await.map_err(StageError::Write)?;
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
    if Sha256::from_bytes(hasher.finalize().into()) != file.sha256 {
        return Err(StageError::Digest);
    }
    // Once, before the file replaces anything: a crash after the swap
    // must not leave a truncated executable in the app's place.
    out.sync_all().await.map_err(StageError::Write)?;
    Ok(())
}

/// Ask for `file` from byte `have` on, and leave `out` (with `hasher`)
/// ready for the body: at `have` when the server sent the rest, emptied
/// when it sent the whole file. Returns the response and where its body
/// starts.
async fn rest(
    client: &FeedClient,
    feed: &FeedUrl,
    file: &File<'_>,
    out: &mut tokio::fs::File,
    hasher: &mut sha2::Sha256,
    have: u64,
) -> Result<(reqwest::Response, u64), StageError> {
    let (response, start) = match client.artifact_from(feed, file.path, have).await {
        Ok(answer) => answer,
        // The server holds less than was kept (416), or sent another
        // range: ask for the whole file instead.
        Err(FetchError::Status { status }) if have > 0 => {
            tracing::info!(%status, kept = have, "the rest was refused: downloading the whole file");
            client
                .artifact_from(feed, file.path, 0)
                .await
                .map_err(StageError::Fetch)?
        }
        Err(error) => return Err(StageError::Fetch(error)),
    };
    if start != have {
        // The server sent the whole file (no ranges): what was kept goes.
        out.set_len(0).await.map_err(StageError::Write)?;
        *hasher = sha2::Sha256::new();
    }
    let _position = out
        .seek(std::io::SeekFrom::Start(start))
        .await
        .map_err(StageError::Write)?;
    tracing::info!(
        status = %response.status(),
        length = ?response.content_length(),
        from = start,
        "the feed answers the download"
    );
    Ok((response, start))
}

/// Hash the bytes an earlier attempt kept in `out` (at most `expected`;
/// more is cut off) and return how many there are.
async fn kept(
    out: &mut tokio::fs::File,
    hasher: &mut sha2::Sha256,
    expected: u64,
) -> Result<u64, StageError> {
    let on_disk = out.metadata().await.map_err(StageError::Write)?.len();
    if on_disk > expected {
        out.set_len(0).await.map_err(StageError::Write)?;
        return Ok(0);
    }
    if on_disk == 0 {
        return Ok(0);
    }
    let _position = out
        .seek(std::io::SeekFrom::Start(0))
        .await
        .map_err(StageError::Write)?;
    let mut buffer = vec![0_u8; CHUNK];
    let mut read_total: u64 = 0;
    while read_total < on_disk {
        let read = out.read(&mut buffer).await.map_err(StageError::Write)?;
        if read == 0 {
            break;
        }
        hasher.update(buffer.get(..read).unwrap_or_default());
        read_total = read_total.saturating_add(u64::try_from(read).unwrap_or(u64::MAX));
    }
    tracing::info!(
        kept = read_total,
        of = expected,
        "going on from an earlier attempt"
    );
    Ok(read_total)
}

/// Unpack the verified gzip file `from` into `to`, which must come out as
/// exactly `size` bytes of digest `sha256`. Blocking: a blocking thread.
fn unpack(from: &Path, to: &Path, size: ByteCount, sha256: Sha256) -> Result<(), StageError> {
    let input = std::fs::File::open(from).map_err(StageError::Write)?;
    let mut decoder = flate2::read::GzDecoder::new(std::io::BufReader::new(input));
    let mut out = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(to)
        .map_err(StageError::Write)?;
    let mut hasher = sha2::Sha256::new();
    let mut buffer = vec![0_u8; CHUNK];
    let mut written: u64 = 0;
    let expected = size.get();
    loop {
        let read = decoder.read(&mut buffer).map_err(StageError::Unpack)?;
        if read == 0 {
            break;
        }
        written = written.saturating_add(u64::try_from(read).unwrap_or(u64::MAX));
        // A signed file cannot be a bomb, but the bound costs nothing.
        if written > expected {
            return Err(StageError::TooLarge);
        }
        let piece = buffer.get(..read).unwrap_or_default();
        hasher.update(piece);
        out.write_all(piece).map_err(StageError::Write)?;
    }
    if written != expected {
        return Err(StageError::Short {
            received: written,
            expected,
        });
    }
    if Sha256::from_bytes(hasher.finalize().into()) != sha256 {
        return Err(StageError::Digest);
    }
    out.sync_all().map_err(StageError::Write)?;
    Ok(())
}
