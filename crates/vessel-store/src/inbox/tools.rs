//! Bounded tool display snapshots, separate from raw output and event retention.
use crate::codec::{decode, encode, seq_to_sql};
use crate::error::{Column, StoreError};
use rusqlite::{Connection, OptionalExtension as _, params};
use vessel_proto::inbox::InboxDetail;
use vessel_proto::timeline::ToolCallScope;
use vessel_proto::{EventSeq, SessionId, ToolCall, ToolCallId, ToolCallUpdate};

pub(super) fn get(
    connection: &Connection,
    session: SessionId,
    id: &ToolCallId,
    scope: Option<&ToolCallScope>,
) -> Result<(Option<ToolCall>, InboxDetail), StoreError> {
    let scope = scope.map(encode).transpose()?.unwrap_or_default();
    let cached: Option<(String, String)> = connection
        .query_row(
            "SELECT tool,detail FROM inbox_tools WHERE session=?1 AND id=?2 AND scope=?3",
            params![session.to_string(), id.as_str(), &scope],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let row = match cached {
        Some(row) => Some(row),
        None => connection.query_row("SELECT json_extract(item,'$.tool'),json_quote(json_extract(item,'$.detail')) FROM inbox WHERE session=?1 AND json_extract(item,'$.kind')='permission' AND json_extract(item,'$.request.tool_call')=?2 AND coalesce(json_extract(item,'$.request.tool_scope'),'')=?3 AND json_type(item,'$.tool')='object' LIMIT 1", params![session.to_string(), id.as_str(), &scope], |row| Ok((row.get(0)?, row.get(1)?))).optional()?,
    };
    match row {
        Some((tool, detail)) => Ok((
            Some(decode(session, Column::Inbox, &tool)?),
            decode(session, Column::Inbox, &detail)?,
        )),
        None => Ok((None, InboxDetail::InSession)),
    }
}

pub(super) fn put(
    connection: &Connection,
    session: SessionId,
    seq: EventSeq,
    tool: ToolCall,
) -> Result<(), StoreError> {
    put_detail(connection, session, seq, tool, InboxDetail::Complete)
}

// A partial update cannot restore omitted content. Preserve that fact before
// encoding and budget admission, rather than adding its bytes afterwards.
fn put_detail(
    connection: &Connection,
    session: SessionId,
    seq: EventSeq,
    mut tool: ToolCall,
    mut detail: InboxDetail,
) -> Result<(), StoreError> {
    if encode(&tool)?.len() > 32_768 {
        tool.raw_input = None;
        tool.detail = None;
        tool.failure = None;
        tool.agent_message = None;
        tool.content.clear();
        tool.locations.clear();
        tool.title = tool.title.chars().take(4096).collect();
        detail = InboxDetail::InSession;
    }
    // An opaque id cannot be shortened. An unretainable replacement must
    // also invalidate the old snapshot, or a pending approval would present
    // the superseded title/content as current and complete.
    if encode(&tool)?.len() > 32_768 {
        return defer_tool(connection, session, &tool.id, tool.scope.as_deref());
    }
    let tick: i64 = connection.query_row(
        "UPDATE inbox_meta SET tool_clock=tool_clock+1 WHERE singleton=1 RETURNING tool_clock",
        [],
        |row| row.get(0),
    )?;
    let scope = tool
        .scope
        .as_deref()
        .map(encode)
        .transpose()?
        .unwrap_or_default();
    let _rows = connection.execute("INSERT INTO inbox_tools(session,id,seq,tick,tool,detail,scope) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(session,id,scope) DO UPDATE SET seq=excluded.seq,tick=excluded.tick,tool=excluded.tool,detail=excluded.detail", params![session.to_string(), tool.id.as_str(), seq_to_sql(seq)?, tick, encode(&tool)?, encode(&detail)?, &scope])?;
    let _old = connection.execute("DELETE FROM inbox_tools WHERE session=?1 AND (id,scope) IN (SELECT id,scope FROM inbox_tools WHERE session=?1 ORDER BY seq DESC LIMIT -1 OFFSET 128)", [session.to_string()])?;
    let _global = connection.execute("DELETE FROM inbox_tools WHERE (session,id,scope) IN (SELECT session,id,scope FROM inbox_tools ORDER BY tick DESC LIMIT -1 OFFSET 512)", [])?;
    // Keep changed details inside both budgets. Above them the row remains
    // pending by its exact request id and explicitly opens the session.
    // SQLite caches an uncorrelated scalar subquery inside one UPDATE.
    // Step exact rows separately so each trigger's pool accounting is visible
    // to the next admission; one tool may have several pending approvals.
    let cursors = connection.prepare_cached("SELECT cursor FROM inbox WHERE session=?1 AND json_extract(item,'$.kind')='permission' AND json_extract(item,'$.request.tool_call')=?2 AND coalesce(json_extract(item,'$.request.tool_scope'),'')=?3 ORDER BY cursor")?
        .query_map(params![session.to_string(), tool.id.as_str(), &scope], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let encoded = encode(&tool)?;
    let detail = encode(&detail)?;
    for cursor in cursors {
        let _pending = connection.execute(
            "UPDATE inbox SET item=CASE
               WHEN length(CAST(json_set(item,'$.tool',json(?2),'$.detail',json(?3)) AS BLOB))>262144
                 OR (SELECT pending_bytes FROM inbox_meta)-length(CAST(item AS BLOB))
                     +length(CAST(json_set(item,'$.tool',json(?2),'$.detail',json(?3)) AS BLOB))>33554432
               THEN CASE WHEN json_type(item,'$.request.tool_call')='text'
                   AND json_extract(item,'$.request.plan_proposal') IS NULL
                   AND coalesce(json_extract(item,'$.request.delivery_state'),'pending')='pending'
                THEN json_object('kind','in_session','request',json_object('kind','permission','id',json_extract(item,'$.request.id')))
                ELSE json_object('kind','in_session','request',json_object('kind','permission','id',json_extract(item,'$.request.id')),
                   'permission',json_object('plan_proposal',json_extract(item,'$.request.plan_proposal'),
                   'delivery_state',coalesce(json_extract(item,'$.request.delivery_state'),'pending'))) END
               ELSE json_set(item,'$.tool',json(?2),'$.detail',json(?3)) END
             WHERE cursor=?1",
            params![cursor, &encoded, &detail],
        )?;
    }
    Ok(())
}

pub(super) fn update(
    connection: &Connection,
    session: SessionId,
    seq: EventSeq,
    update: &ToolCallUpdate,
) -> Result<(), StoreError> {
    let (Some(mut tool), prior) = get(connection, session, &update.id, update.scope.as_deref())?
    else {
        return Ok(());
    };
    if let Some(detail) = &update.detail {
        tool.detail = Some(match tool.detail.take() {
            Some(previous) => Box::new((*previous).updated(Some((**detail).clone()))),
            None => detail.clone(),
        });
    }
    if let Some(failure) = &update.failure {
        tool.failure = Some(failure.clone());
    }
    if let Some(source) = &update.agent_message {
        tool.agent_message = Some(source.clone());
    }
    if let Some(title) = &update.title {
        tool.title.clone_from(title);
    }
    if let Some(kind) = update.kind {
        tool.kind = kind;
    }
    if let Some(status) = update.status {
        tool.status = status;
    }
    if let Some(content) = &update.content {
        tool.content.clone_from(content);
    }
    if let Some(locations) = &update.locations {
        tool.locations.clone_from(locations);
    }
    if let Some(input) = &update.raw_input {
        tool.raw_input = Some(input.clone());
    }
    put_detail(connection, session, seq, tool, prior)
}

fn defer_tool(
    connection: &Connection,
    session: SessionId,
    id: &ToolCallId,
    scope: Option<&ToolCallScope>,
) -> Result<(), StoreError> {
    let scope = scope.map(encode).transpose()?.unwrap_or_default();
    let _cached = connection.execute(
        "DELETE FROM inbox_tools WHERE session=?1 AND id=?2 AND scope=?3",
        params![session.to_string(), id.as_str(), &scope],
    )?;
    // Every replacement shrinks the pool, so this needs no row-by-row
    // admission. The existing triggers release its exact complete bytes.
    let _pending = connection.execute(
        "UPDATE inbox SET item=CASE WHEN json_type(item,'$.request.tool_call')='text'
                   AND json_extract(item,'$.request.plan_proposal') IS NULL
                   AND coalesce(json_extract(item,'$.request.delivery_state'),'pending')='pending'
                THEN json_object('kind','in_session','request',json_object('kind','permission','id',json_extract(item,'$.request.id')))
                ELSE json_object('kind','in_session','request',json_object('kind','permission','id',json_extract(item,'$.request.id')),
                   'permission',json_object('plan_proposal',json_extract(item,'$.request.plan_proposal'),
                   'delivery_state',coalesce(json_extract(item,'$.request.delivery_state'),'pending'))) END
         WHERE session=?1 AND json_extract(item,'$.kind')='permission'
           AND json_extract(item,'$.request.tool_call')=?2
           AND coalesce(json_extract(item,'$.request.tool_scope'),'')=?3",
        params![session.to_string(), id.as_str(), &scope],
    )?;
    Ok(())
}
