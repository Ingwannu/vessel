//! One-time reconstruction of retained pre-inbox outcomes.
use super::{Connection, StoreError, observe, session_id_from_sql};
/// Existing ended sessions have no pending requests after startup recovery.
/// Backfill only retained, unarchived attention; never replay every old log.
/// The migration transaction includes this, so it cannot run twice after a crash.
pub(crate) fn backfill(connection: &Connection) -> Result<(), StoreError> {
    // History before a fresh turn or rewind is no longer the actor's
    // current branch. In particular, recovery must not revive its old limit.
    let mut query = connection.prepare(
        "WITH current AS (
           SELECT s.*, COALESCE((SELECT MAX(seq) FROM events
             WHERE session_id=s.id AND (
               json_extract(event,'$.type')='rewound' OR
               (json_extract(event,'$.type')='agent' AND json_extract(event,'$.event.event')='turn_started')
             )), -1) AS boundary
           FROM sessions s WHERE s.live=0 AND s.archived_at IS NULL
         )
         SELECT e.seq,e.at,e.event,s.id
         FROM current s JOIN events e ON e.session_id=s.id AND e.seq=COALESCE(
           (SELECT MAX(seq) FROM events WHERE session_id=s.id AND seq>s.boundary
             AND json_extract(event,'$.type')='agent'
             AND json_extract(event,'$.event.event') IN ('turn_ended','turn_failed')),
           s.attention_seq)
         ORDER BY s.created_at",
    )?;
    let mut rows = query.query([])?;
    while let Some(row) = rows.next()? {
        let session = session_id_from_sql(&row.get::<_, String>(3)?)?;
        let (record, _) = crate::codec::record_from_row(session, row)?;
        observe(connection, session, &record)?;
    }
    Ok(())
}
