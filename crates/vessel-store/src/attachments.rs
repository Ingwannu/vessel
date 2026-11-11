//! The files sent with a session's prompts (protocol 10, ADR 0025), as far
//! as the store's own operations need them: a session's folder goes when
//! its row does, whether `session.delete` or retention removed it.
//!
//! Must not: remove anything but a session's own folder under the data
//! directory's `attachments/`, or remove it before the row's deletion is
//! committed (a failed commit keeps the session, so it keeps its files).
//!
//! [Decision Log]
//! - Purpose: a deleted or pruned session must not leave its files behind
//!   (review item 10); the files may be large and may be private.
//! - Alternatives: leave them to a sweep at the node's start (files then
//!   outlive their session for as long as the node runs); remove them on
//!   the tokio blocking pool (the writer would answer before the files are
//!   gone, so a caller of `session.delete` could still find them).
//! - Choice: the writer removes them itself, right after the commit that
//!   removed the rows and before it answers. It is a plain thread, so the
//!   blocking calls are its to make; removals are rare (a deletion, or a
//!   session pruned when another ends past the cap).
//! - Cost: the writer's next batch waits for a folder's removal, a few
//!   unlinks; a failure is logged and leaves an orphan for the start-up
//!   sweep (the attachments slice).

use std::io;
use std::path::Path;

use vessel_proto::SessionId;

/// Remove `session`'s attachment folder under `root` (the data
/// directory's `attachments/`), if it has one. A link in its place is
/// removed, not followed (`remove_dir_all`'s rule), so nothing outside
/// the folder is ever touched. Failures are logged: the session is gone
/// either way.
pub(crate) fn remove_folder(root: &Path, session: SessionId) {
    // A session id is a UUID: one path component, never `..`.
    let folder = root.join(session.to_string());
    match std::fs::remove_dir_all(&folder) {
        Ok(()) => tracing::debug!(%session, "removed the session's attachments"),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => tracing::warn!(
            %session,
            path = %folder.display(),
            %error,
            "could not remove a removed session's attachments"
        ),
    }
}
