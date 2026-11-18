//! `node.log` (ADR 0111, TRACKER 569): the last lines of the node's own log,
//! for its administrators (`admin`, never forwarded, refused to agents by
//! `workspace::policy`).
//!
//! [Decision Log]
//! - Which file: the one this node's standard error is. A node started in
//!   the background writes `logs/vesseld.log` and a launchd service
//!   `logs/vessel-service.log`; a systemd service writes to its journal and
//!   a node started in a terminal to that terminal. Comparing the device and
//!   inode of `/dev/fd/2` with the two files tells which, without trusting
//!   a file merely for existing (an old one stays after a node moves to a
//!   service). Elsewhere than Unix, `vesseld.log` when it exists.
//! - Alternatives: a flag from whoever starts the node (installed services
//!   would not pass it); the newest of the two files (wrong after a move to
//!   the journal).
//! - Bounds: `vessel_proto::diagnostics`'s (`MAX_LOG_TAIL_BYTES` read from
//!   the end, the first partial line dropped, each line cut at
//!   `MAX_LOG_LINE_BYTES`), so a reply is at most about 64 KiB.

use std::io::{IsTerminal as _, Read as _, Seek as _, SeekFrom};
use std::path::{Path, PathBuf};

use vessel_proto::agents::bounded_text;
use vessel_proto::diagnostics::{
    DEFAULT_LOG_LINES, LogSink, MAX_LOG_LINE_BYTES, MAX_LOG_LINES, MAX_LOG_TAIL_BYTES, NodeLog,
    ReadNodeLog,
};
use vessel_proto::{ErrorCode, WireError};

use super::agent_diagnostic::folded;
use super::{Connection, NodeService};

/// The stream a node's log goes to, as a file the system can stat.
const STANDARD_ERROR: &str = "/dev/fd/2";

impl NodeService {
    /// `node.log`.
    pub(crate) async fn node_log(
        &self,
        _: &Connection,
        request: ReadNodeLog,
    ) -> Result<NodeLog, WireError> {
        let lines = request.lines.unwrap_or(DEFAULT_LOG_LINES);
        if lines == 0 || lines > MAX_LOG_LINES {
            return Err(WireError::new(
                ErrorCode::InvalidRequest,
                format!("a log is read 1 to {MAX_LOG_LINES} lines at a time"),
            ));
        }
        let files = self.diagnostics.data_dir().node_log_files();
        tokio::task::spawn_blocking(move || read(Path::new(STANDARD_ERROR), &files, lines))
            .await
            .map_err(|_join| unreadable())?
            .map_err(|error| {
                tracing::info!(%error, "the node's log was not read");
                unreadable()
            })
    }
}

fn unreadable() -> WireError {
    WireError::new(ErrorCode::Internal, "the node's log could not be read")
}

/// The log `stream` writes to among `files`, with its last `lines`.
fn read(stream: &Path, files: &[PathBuf], lines: u32) -> std::io::Result<NodeLog> {
    let Some(path) = log_file(stream, files) else {
        let writes_to = if std::io::stderr().is_terminal() {
            LogSink::Terminal
        } else {
            LogSink::Elsewhere
        };
        return Ok(NodeLog::NoFile { writes_to });
    };
    let (lines, cut) = tail(&path, lines)?;
    Ok(NodeLog::File {
        path: folded(&path),
        lines,
        cut,
    })
}

/// The file among `files` that `stream` (a `/dev/fd/N`) is.
#[cfg(unix)]
pub(super) fn log_file(stream: &Path, files: &[PathBuf]) -> Option<PathBuf> {
    use std::os::unix::fs::MetadataExt as _;
    let stream = std::fs::metadata(stream).ok()?;
    files
        .iter()
        .find(|file| {
            std::fs::metadata(file)
                .is_ok_and(|file| file.dev() == stream.dev() && file.ino() == stream.ino())
        })
        .cloned()
}

/// `vesseld.log` (the first of `files`) when it exists.
#[cfg(not(unix))]
pub(super) fn log_file(_stream: &Path, files: &[PathBuf]) -> Option<PathBuf> {
    files.first().filter(|file| file.exists()).cloned()
}

/// The last `lines` of `path`, read from at most its last
/// `MAX_LOG_TAIL_BYTES`; whether anything was left out or cut.
pub(super) fn tail(path: &Path, lines: u32) -> std::io::Result<(Vec<String>, bool)> {
    let mut file = std::fs::File::open(path)?;
    let start = file.metadata()?.len().saturating_sub(MAX_LOG_TAIL_BYTES);
    let _at = file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::new();
    let _read = file.take(MAX_LOG_TAIL_BYTES).read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes);
    let mut kept: Vec<&str> = text.lines().collect();
    let mut cut = false;
    if start > 0 && !kept.is_empty() {
        // The first line read began before the bytes read.
        let _partial = kept.remove(0);
        cut = true;
    }
    let wanted = usize::try_from(lines).unwrap_or(usize::MAX);
    let excess = kept.len().saturating_sub(wanted);
    if excess > 0 {
        kept = kept.split_off(excess);
        cut = true;
    }
    let lines = kept
        .into_iter()
        .map(|line| {
            let bounded = bounded_text(line, MAX_LOG_LINE_BYTES);
            cut |= bounded.len() < line.len();
            bounded
        })
        .collect();
    Ok((lines, cut))
}

#[cfg(test)]
mod tests;
