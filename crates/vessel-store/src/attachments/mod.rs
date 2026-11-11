//! The files sent with a session's prompts (protocol 10, ADR 0025): their
//! rows (`attachments`, schema 7), their bytes under the data directory's
//! `attachments/<session>/<id>/<name>` (`files`), and their removal with
//! their session.
//!
//! Must not: remove anything but a session's own folder under the data
//! directory's `attachments/`, or remove it before the row's deletion is
//! committed (a failed commit keeps the session, so it keeps its files);
//! keep an attachment for a session that has ended; read one back other
//! than as `files` allows.
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
//! - Keeping one: the bytes are put in place and checked (`files::place`,
//!   on the blocking pool: hashing up to 50 MiB must not hold the writer),
//!   then the row is inserted by the writer, which refuses a session that
//!   is not live any more; a refused row takes its file back. So a row
//!   never names a file that was not checked, and a file without a row
//!   lives at most until the next start, whose sweep ([`sweep`]) removes
//!   every folder no row names.
//! - Cost: the writer's next batch waits for a folder's removal, a few
//!   unlinks; a failure is logged and leaves an orphan for the start-up
//!   sweep.

pub(crate) mod files;

use std::collections::HashSet;
use std::ffi::OsString;
use std::io;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension as _, Row, params};
use vessel_proto::{
    AttachmentId, AttachmentKind, AttachmentName, AttachmentRef, BlobHash, SessionId,
};

use vessel_proto::conversation::MimeType;

use crate::error::StoreError;

/// An attachment as the store keeps it: its reference as `attachment.finish`
/// returned it, whose session it is, and the SHA-256 of its bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptAttachment {
    /// The session it was sent to.
    pub session: SessionId,
    /// What a prompt names: id, name, kind, type, size.
    pub reference: AttachmentRef,
    /// The SHA-256 of its bytes, as received.
    pub sha256: BlobHash,
}

/// How many bytes of attachments are kept, for the node's limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AttachmentUsage {
    /// All sessions together.
    pub node: u64,
    /// The session asked about.
    pub session: u64,
}

/// The columns [`from_row`] reads, in order.
const COLUMNS: &str = "session_id, id, name, kind, mime, size, sha256";

/// Insert `kept`'s row; its session must be stored and live.
pub(crate) fn insert(
    connection: &Connection,
    kept: &KeptAttachment,
    at: i64,
) -> Result<(), StoreError> {
    let session = kept.session.to_string();
    let live: Option<bool> = connection
        .prepare_cached("SELECT live FROM sessions WHERE id = ?1")?
        .query_row([&session], |row| row.get(0))
        .optional()?;
    match live {
        None => return Err(StoreError::UnknownSession(kept.session)),
        Some(false) => return Err(StoreError::SessionEnded(kept.session)),
        Some(true) => {}
    }
    let reference = &kept.reference;
    let size = i64::try_from(reference.size)
        .map_err(|_too_big| StoreError::OutOfRange(crate::error::Quantity::Bytes))?;
    let _inserted = connection
        .prepare_cached(&format!(
            "INSERT INTO attachments ({COLUMNS}, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
        ))?
        .execute(params![
            session,
            reference.id.to_string(),
            reference.name.as_str(),
            kind_to_sql(reference.kind),
            reference.mime.as_str(),
            size,
            kept.sha256.as_bytes().as_slice(),
            at,
        ])?;
    Ok(())
}

/// `session`'s attachment `id`, if the store keeps it.
pub(crate) fn select(
    connection: &Connection,
    session: SessionId,
    id: AttachmentId,
) -> Result<Option<KeptAttachment>, StoreError> {
    let row = connection
        .prepare_cached(&format!(
            "SELECT {COLUMNS} FROM attachments WHERE id = ?1 AND session_id = ?2"
        ))?
        .query_row(params![id.to_string(), session.to_string()], |row| {
            Ok(from_row(row))
        })
        .optional()?;
    row.transpose()
}

/// The bytes kept, on the node and for `session`.
pub(crate) fn usage(
    connection: &Connection,
    session: SessionId,
) -> Result<AttachmentUsage, StoreError> {
    let (node, session): (i64, i64) = connection
        .prepare_cached(
            "SELECT COALESCE(SUM(size), 0), COALESCE(SUM(CASE WHEN session_id = ?1 THEN size END), 0) \
             FROM attachments",
        )?
        .query_row([session.to_string()], |row| Ok((row.get(0)?, row.get(1)?)))?;
    let bytes = |value: i64| {
        u64::try_from(value)
            .map_err(|_negative| StoreError::OutOfRange(crate::error::Quantity::Bytes))
    };
    Ok(AttachmentUsage {
        node: bytes(node)?,
        session: bytes(session)?,
    })
}

fn from_row(row: &Row<'_>) -> Result<KeptAttachment, StoreError> {
    let text = |index: usize| -> Result<String, StoreError> { Ok(row.get(index)?) };
    let id_text = text(1)?;
    let corrupt = |what: &'static str| StoreError::CorruptAttachment {
        id: id_text.clone(),
        what,
    };
    let session = text(0)?.parse().map_err(|_bad| corrupt("session"))?;
    let id = id_text.parse().map_err(|_bad| corrupt("id"))?;
    let name: AttachmentName = text(2)?.parse().map_err(|_bad| corrupt("name"))?;
    let kind = kind_from_sql(&text(3)?).ok_or_else(|| corrupt("kind"))?;
    let mime: MimeType = text(4)?.parse().map_err(|_bad| corrupt("type"))?;
    let size: i64 = row.get(5)?;
    let size = u64::try_from(size).map_err(|_negative| corrupt("size"))?;
    let sha256: Vec<u8> = row.get(6)?;
    let sha256 = <[u8; 32]>::try_from(sha256.as_slice()).map_err(|_length| corrupt("hash"))?;
    Ok(KeptAttachment {
        session,
        reference: AttachmentRef {
            id,
            name,
            kind,
            mime,
            size,
        },
        sha256: BlobHash::from_bytes(sha256),
    })
}

const fn kind_to_sql(kind: AttachmentKind) -> &'static str {
    match kind {
        AttachmentKind::Image => "image",
        AttachmentKind::Audio => "audio",
        AttachmentKind::File => "file",
    }
}

fn kind_from_sql(text: &str) -> Option<AttachmentKind> {
    match text {
        "image" => Some(AttachmentKind::Image),
        "audio" => Some(AttachmentKind::Audio),
        "file" => Some(AttachmentKind::File),
        _ => None,
    }
}

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

/// Remove whatever under `root` no row names: a session's folder whose
/// session is gone, an attachment's folder whose row is (a file placed
/// whose row was never committed, a removal that failed). Runs at open,
/// before the node serves anything, so no session's agent runs yet and
/// nothing is swapped under it; anything that is not a folder, links
/// included, is removed rather than looked into. Failures are logged.
pub(crate) fn sweep(connection: &Connection, root: &Path) -> Result<(), StoreError> {
    let kept: HashSet<(String, String)> = connection
        .prepare("SELECT session_id, id FROM attachments")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let sessions: HashSet<String> = connection
        .prepare("SELECT id FROM sessions")?
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    for (name, folder) in entries(root) {
        let session = name.to_str().filter(|session| sessions.contains(*session));
        let (Some(session), true) = (session, folder) else {
            discard(&root.join(&name), folder);
            continue;
        };
        let inside = root.join(&name);
        for (id, folder) in entries(&inside) {
            let named = id
                .to_str()
                .is_some_and(|id| kept.contains(&(session.to_owned(), id.to_owned())));
            if !(folder && named) {
                discard(&inside.join(id), folder);
            }
        }
    }
    Ok(())
}

/// The names under `dir` and whether each is a folder (not following a
/// link); nothing when `dir` is not there.
fn entries(dir: &Path) -> Vec<(OsString, bool)> {
    let listed = match std::fs::read_dir(dir) {
        Ok(listed) => listed,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Vec::new(),
        Err(error) => {
            tracing::warn!(path = %dir.display(), %error, "could not list attachments to sweep");
            return Vec::new();
        }
    };
    listed
        .filter_map(Result::ok)
        .map(|entry| {
            let folder = entry.file_type().is_ok_and(|kind| kind.is_dir());
            (entry.file_name(), folder)
        })
        .collect()
}

fn discard(path: &Path, folder: bool) {
    let removed = if folder {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
    match removed {
        Ok(()) => tracing::info!(path = %path.display(), "removed an attachment no session keeps"),
        Err(error) => tracing::warn!(
            path = %path.display(),
            %error,
            "could not remove an attachment no session keeps"
        ),
    }
}
