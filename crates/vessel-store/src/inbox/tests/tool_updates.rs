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
    request.tool_call = Some(tool_id.clone());
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
    )
    .unwrap();
    append(
        &db,
        session,
        2,
        AgentEvent::ToolCallUpdate(vessel_proto::ToolCallUpdate {
            scope: None,
            id: tool_id.clone(),
            title: Some("t".repeat(4096)),
            kind: None,
            status: None,
            content: None,
            locations: None,
            raw_input: None,
            raw_output: None,

            detail: None,
            failure: None,
            agent_message: None,
        }),
    );
    let page = super::super::list(&db, ListInbox::default()).unwrap();
    let [current] = page.entries.as_slice() else {
        panic!("the request must remain discoverable");
    };
    assert!(
        matches!(current.item, InboxItem::InSession {
        request: InboxRequest::Permission { id }, ..
    } if id == request_id),
        "a superseded tool must never remain complete"
    );
    assert_eq!(current.key, old.key);
    assert_eq!(current.cursor, old.cursor);
    assert_eq!(current.read, InboxRead::Read);
    assert_eq!(page.counts.waiting.get(), 1);
    assert_eq!(page.counts.unread.get(), 0);
    let (cached, detail) = super::super::tools::get(&db, session, &tool_id, None).unwrap();
    assert!(cached.is_none());
    assert_eq!(detail, InboxDetail::InSession);
    let bytes: i64 = db
        .query_row("SELECT pending_bytes FROM inbox_meta", [], |row| row.get(0))
        .unwrap();
    assert_eq!(bytes, 0);
}

#[test]
fn late_updates_cannot_replace_another_turns_pending_permission_details() {
    use vessel_proto::timeline::ToolCallScope;
    let db = database();
    let session = session(&db);
    let first_scope = ToolCallScope::ProviderTurn {
        thread: "thread-1".parse().unwrap(),
        turn: "turn-1".parse().unwrap(),
    };
    let second_scope = ToolCallScope::ProviderTurn {
        thread: "thread-1".parse().unwrap(),
        turn: "turn-2".parse().unwrap(),
    };
    let mut first = tool();
    first.scope = Some(Box::new(first_scope.clone()));
    first.title = "first turn".into();
    let mut first_request = permission();
    first_request.tool_scope.clone_from(&first.scope);
    let first_id = first_request.id;
    append(&db, session, 0, AgentEvent::ToolCall(first));
    append(
        &db,
        session,
        1,
        AgentEvent::PermissionRequested(first_request),
    );
    let mut second = tool();
    second.scope = Some(Box::new(second_scope.clone()));
    second.title = "second turn".into();
    let mut second_request = permission();
    second_request.tool_scope.clone_from(&second.scope);
    let second_id = second_request.id;
    append(&db, session, 2, AgentEvent::ToolCall(second));
    append(
        &db,
        session,
        3,
        AgentEvent::PermissionRequested(second_request),
    );
    let update = serde_json::from_value(serde_json::json!({
        "id":tool().id, "scope":first_scope, "title":"late first update"
    }))
    .unwrap();
    append(&db, session, 4, AgentEvent::ToolCallUpdate(update));
    let page = super::super::list(&db, ListInbox::default()).unwrap();
    for entry in page.entries {
        let InboxItem::Permission {
            request,
            tool: Some(tool),
            ..
        } = entry.item
        else {
            panic!("complete permission")
        };
        if request.id == first_id {
            assert_eq!(tool.title, "late first update");
        } else {
            assert_eq!(request.id, second_id);
            assert_eq!(tool.title, "second turn");
        }
    }
    let (second, _) =
        super::super::tools::get(&db, session, &tool().id, Some(&second_scope)).unwrap();
    assert_eq!(second.unwrap().title, "second turn");
}
