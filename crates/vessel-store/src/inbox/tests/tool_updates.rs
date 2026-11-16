//! Updates cannot leave an old complete snapshot behind a bounded fallback.
use super::*;
use vessel_proto::inbox::InboxRequest;

#[test]
fn an_unretainable_update_invalidates_the_prior_tool_snapshot() {
    let db = database();
    let session = session(&db);
    let mut call = tool();
    call.id = ToolCallId::new("x".repeat(30_000));
    let tool_id = call.id.clone();
    let mut request = permission();
    request.tool_call = tool_id.clone();
    let request_id = request.id;
    append(&db, session, 0, AgentEvent::ToolCall(call));
    append(&db, session, 1, AgentEvent::PermissionRequested(request));
    let old = super::super::list(&db, ListInbox::default())
        .unwrap()
        .entries
        .remove(0);
    assert!(matches!(
        old.item,
        InboxItem::Permission {
            detail: InboxDetail::Complete,
            ..
        }
    ));
    super::super::mark(
        &db,
        &MarkInbox {
            key: old.key,
            read: InboxRead::Read,
        },
        crate::Residence::Stored,
    )
    .unwrap();
    append(
        &db,
        session,
        2,
        AgentEvent::ToolCallUpdate(vessel_proto::ToolCallUpdate {
            id: tool_id.clone(),
            title: Some("t".repeat(4096)),
            kind: None,
            status: None,
            content: None,
            locations: None,
            raw_input: None,
            raw_output: None,
        }),
    );
    let page = super::super::list(&db, ListInbox::default()).unwrap();
    let [current] = page.entries.as_slice() else {
        panic!("the request must remain discoverable");
    };
    assert!(
        matches!(current.item, InboxItem::InSession {
        request: InboxRequest::Permission { id }
    } if id == request_id),
        "a superseded tool must never remain complete"
    );
    assert_eq!(current.key, old.key);
    assert_eq!(current.cursor, old.cursor);
    assert_eq!(current.read, InboxRead::Read);
    assert_eq!(page.counts.waiting.get(), 1);
    assert_eq!(page.counts.unread.get(), 0);
    let (cached, detail) = super::super::tools::get(&db, session, &tool_id).unwrap();
    assert!(cached.is_none());
    assert_eq!(detail, InboxDetail::InSession);
    let bytes: i64 = db
        .query_row("SELECT pending_bytes FROM inbox_meta", [], |row| row.get(0))
        .unwrap();
    assert_eq!(bytes, 0);
}
