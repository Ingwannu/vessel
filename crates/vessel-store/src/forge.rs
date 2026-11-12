//! What the node keeps of a forge (protocol 11, schema 10, ADR 0029): which
//! sessions have feedback on, and the signatures of the events already
//! sent to each.
//!
//! [Decision Log]
//! - Purpose: feedback is off by default and turned on per session by a
//!   person, and each event is sent once: both must survive the node
//!   stopping, or a restart would send every unresolved comment again.
//! - Constraint: the pull request itself is the forge's and is found
//!   again by its branch; keeping a copy would only go stale.
//! - Choice: two tables keyed by session and deleted with it (`ON DELETE
//!   CASCADE`), so retention and deletion need no step of their own. At
//!   most [`MAX_SENT`] signatures per session: an insertion past it drops
//!   the oldest, and the node first forgets those its last read no longer
//!   saw ([`ForgeWrite::Forget`]). A signature is the node's own text
//!   (kind, the forge's id, the head), opaque here.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use vessel_proto::SessionId;

use crate::codec::time_to_sql;
use crate::error::StoreError;

/// The most signatures kept per session.
pub const MAX_SENT: usize = 1000;

/// A session's feedback, as stored.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoredFeedback {
    /// A person turned it on.
    pub on: bool,
    /// The signatures of the events sent, oldest first.
    pub sent: Vec<String>,
}

/// A change to the forge's record.
#[derive(Debug)]
pub(crate) enum ForgeWrite {
    /// Turn a session's feedback on or off.
    Feedback { session: SessionId, on: bool },
    /// Record signatures sent at `at`, keeping at most [`MAX_SENT`].
    Sent {
        session: SessionId,
        signatures: Vec<String>,
        at: DateTime<Utc>,
    },
    /// Forget signatures (events the last read no longer saw).
    Forget {
        session: SessionId,
        signatures: Vec<String>,
    },
}

/// Apply one change. Writer side.
pub(crate) fn apply(connection: &Connection, write: &ForgeWrite) -> Result<(), StoreError> {
    match write {
        ForgeWrite::Feedback { session, on: true } => {
            let _inserted = connection
                .prepare_cached(
                    "INSERT INTO forge_feedback_on (session_id) VALUES (?1)
                     ON CONFLICT (session_id) DO NOTHING",
                )?
                .execute([session.to_string()])?;
        }
        ForgeWrite::Feedback { session, on: false } => {
            let _deleted = connection
                .prepare_cached("DELETE FROM forge_feedback_on WHERE session_id = ?1")?
                .execute([session.to_string()])?;
        }
        ForgeWrite::Sent {
            session,
            signatures,
            at,
        } => {
            let at = time_to_sql(*at)?;
            let mut insert = connection.prepare_cached(
                "INSERT INTO forge_feedback_sent (session_id, signature, sent_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT (session_id, signature) DO NOTHING",
            )?;
            for signature in signatures {
                let _inserted = insert.execute(params![session.to_string(), signature, at])?;
            }
            let keep = i64::try_from(MAX_SENT).unwrap_or(i64::MAX);
            let _pruned = connection
                .prepare_cached(
                    "DELETE FROM forge_feedback_sent WHERE session_id = ?1 AND signature NOT IN (
                         SELECT signature FROM forge_feedback_sent WHERE session_id = ?1
                         ORDER BY sent_at DESC, signature LIMIT ?2)",
                )?
                .execute(params![session.to_string(), keep])?;
        }
        ForgeWrite::Forget {
            session,
            signatures,
        } => {
            let mut delete = connection.prepare_cached(
                "DELETE FROM forge_feedback_sent WHERE session_id = ?1 AND signature = ?2",
            )?;
            for signature in signatures {
                let _deleted = delete.execute(params![session.to_string(), signature])?;
            }
        }
    }
    Ok(())
}

/// `session`'s feedback. Reader side.
pub(crate) fn load(
    connection: &Connection,
    session: SessionId,
) -> Result<StoredFeedback, StoreError> {
    let on = connection
        .prepare_cached("SELECT 1 FROM forge_feedback_on WHERE session_id = ?1")?
        .exists([session.to_string()])?;
    let mut statement = connection.prepare_cached(
        "SELECT signature FROM forge_feedback_sent WHERE session_id = ?1
         ORDER BY sent_at, signature",
    )?;
    let sent = statement
        .query_map([session.to_string()], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(StoredFeedback { on, sent })
}
