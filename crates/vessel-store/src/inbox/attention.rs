//! Indexed session rollups reuse the same inbox rows as list/get/mark.
//! No head-page inference and no second read-state engine lives here.
use crate::codec::session_id_from_sql;
use crate::{Quantity, StoreError};
use rusqlite::Connection;
use vessel_proto::SessionId;
use vessel_proto::inbox::InboxCount;
use vessel_proto::inbox_attention::SessionAttention;

pub(crate) fn read(
    connection: &Connection,
    sessions: &[SessionId],
) -> Result<Vec<SessionAttention>, StoreError> {
    let mut query = connection.prepare_cached(
        "SELECT s.id, COALESCE(SUM(i.seen=0),0), COALESCE(SUM(i.waiting=1),0), COALESCE(SUM(json_extract(i.item,'$.kind')='failed'),0), s.marked_unread
         FROM sessions s LEFT JOIN inbox i ON i.session=s.id
         WHERE s.id=?1 GROUP BY s.id",
    )?;
    let mut attention = Vec::with_capacity(sessions.len());
    for session in sessions {
        let mut rows = query.query([session.to_string()])?;
        if let Some(row) = rows.next()? {
            let count = |value: i64| {
                u64::try_from(value)
                    .map(InboxCount::new)
                    .map_err(|_negative| StoreError::OutOfRange(Quantity::Seq))
            };
            attention.push(SessionAttention {
                session: session_id_from_sql(&row.get::<_, String>(0)?)?,
                unread: count(row.get(1)?)?,
                waiting: count(row.get(2)?)?,
                failed: count(row.get(3)?)?,
                explicit_unread: row.get(4)?,
            });
        }
    }
    Ok(attention)
}
