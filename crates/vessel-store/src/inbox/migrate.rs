//! One-time reconstruction of retained pre-inbox outcomes.
use super::{Connection, StoreError, observe, session_id_from_sql};
/// Existing ended sessions have no pending requests after startup recovery.
/// Backfill only retained, unarchived attention; never replay every old log.
/// The migration transaction includes this, so it cannot run twice after a crash.
pub(crate) fn backfill(connection: &Connection) -> Result<(), StoreError> {
    let mut query = connection.prepare("SELECT e.seq,e.at,e.event,s.id,s.read_seq,s.marked_unread FROM sessions s JOIN events e ON e.session_id=s.id AND e.seq=COALESCE((SELECT MAX(seq) FROM events WHERE session_id=s.id AND json_extract(event,'$.type')='agent' AND json_extract(event,'$.event.event') IN ('turn_ended','turn_failed')),s.attention_seq) WHERE s.live=0 AND s.archived_at IS NULL ORDER BY s.created_at")?;
    let mut rows = query.query([])?;
    while let Some(row) = rows.next()? {
        let session = session_id_from_sql(&row.get::<_, String>(3)?)?;
        let (record, _) = crate::codec::record_from_row(session, row)?;
        observe(connection, session, &record)?;
        let through: Option<i64> = row.get(4)?;
        let unread: bool = row.get(5)?;
        let seq: i64 = row.get(0)?;
        if !unread && through.is_some_and(|read| read > seq) {
            let _rows = connection.execute(
                "UPDATE inbox SET seen=1 WHERE session=?1",
                [session.to_string()],
            )?;
        }
    }
    Ok(())
}
