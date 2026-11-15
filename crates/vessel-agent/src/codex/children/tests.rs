//! Actual native envelopes exercise routing without starting Codex or reading its files.
use super::*;
use serde_json::json;
use vessel_proto::ContentBlock;
fn feed(children: &mut Children, method: &str, params: Value) -> Vec<AgentEvent> {
    children.observe("root", method, &params)
}
fn spawn(sender: &str, child: &str, status: Option<&str>) -> Value {
    let observations = status.map_or_else(
        || json!({}),
        |status| json!({child:{"status":status,"message":null}}),
    );
    json!({"threadId":sender,"item":{"type":"collabAgentToolCall","id":format!("spawn-{child}"),"tool":"spawnAgent","status":"completed","senderThreadId":sender,"receiverThreadIds":[child],"agentsStates":observations,"prompt":"Find the configuration"}})
}
fn messages(events: &[AgentEvent], child: &str) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| {
            if let AgentEvent::ProviderChild(ProviderObservation::Activity {
                child: id,
                activity,
            }) = event
                && id.as_str() == child
                && let AgentEvent::MessageChunk {
                    content: ContentBlock::Text { text },
                } = activity.event()
            {
                Some(text.clone())
            } else {
                None
            }
        })
        .collect()
}
#[test]
fn actual_receivers_are_readonly_children_with_unknown_unreported_status() {
    let mut children = Children::new(false);
    let _observed = feed(
        &mut children,
        "item/completed",
        spawn("root", "child", None),
    );
    let id = "child".parse::<ProviderChildId>().unwrap();
    assert_eq!(children.entries[&id].descriptor.status, None);
    assert_eq!(children.entries[&id].descriptor.created_at, None);
    let events = feed(
        &mut children,
        "item/agentMessage/delta",
        json!({"threadId":"child","itemId":"m","delta":"reply"}),
    );
    assert_eq!(messages(&events, "child"), vec!["reply"]);
    assert!(messages(&events, "root").is_empty());
}
#[test]
fn parent_tool_completion_does_not_complete_a_running_child_and_actual_child_turn_does() {
    let mut children = Children::new(false);
    let _observed = feed(
        &mut children,
        "item/completed",
        spawn("root", "child", Some("running")),
    );
    let id = "child".parse::<ProviderChildId>().unwrap();
    assert_eq!(
        children.entries[&id].descriptor.status,
        Some(ChildStatus::Running)
    );
    let _observed = feed(
        &mut children,
        "turn/completed",
        json!({"threadId":"child","turn":{"id":"t","status":"completed"}}),
    );
    assert_eq!(
        children.entries[&id].descriptor.status,
        Some(ChildStatus::Completed)
    );
}
#[test]
fn nested_routes_use_actual_sender_and_same_item_ids_do_not_merge_siblings() {
    let mut children = Children::new(false);
    let _observed = feed(&mut children, "item/completed", spawn("root", "a", None));
    let _observed = feed(&mut children, "item/completed", spawn("a", "b", None));
    let a = "a".parse::<ProviderChildId>().unwrap();
    let b = "b".parse::<ProviderChildId>().unwrap();
    assert_eq!(children.entries[&b].descriptor.parent, Some(a));
    for id in ["a", "b"] {
        let events = feed(
            &mut children,
            "item/agentMessage/delta",
            json!({"threadId":id,"itemId":"same","delta":id}),
        );
        assert_eq!(messages(&events, id), vec![id]);
    }
    let events = feed(
        &mut children,
        "item/completed",
        json!({"threadId":"b","item":{"type":"agentMessage","id":"same","text":"b"}}),
    );
    assert!(messages(&events, "b").is_empty());
}
#[test]
fn unrelated_threads_cannot_create_routes_or_pollute_an_existing_child() {
    let mut children = Children::new(false);
    let _observed = feed(
        &mut children,
        "item/completed",
        spawn("unrelated", "child", Some("running")),
    );
    assert!(children.entries.is_empty());
    assert!(
        messages(
            &feed(
                &mut children,
                "item/agentMessage/delta",
                json!({"threadId":"unrelated","itemId":"m","delta":"secret-other-thread"})
            ),
            "unrelated"
        )
        .is_empty()
    );
}
#[test]
fn reannouncement_deduplicates_prompts_and_resume_discloses_missing_native_prefix() {
    let mut children = Children::new(true);
    let events = feed(
        &mut children,
        "item/completed",
        spawn("root", "child", Some("pendingInit")),
    );
    assert!(events.iter().any(|event| matches!(
        event,
        AgentEvent::ProviderChild(ProviderObservation::Gap {
            reason: ChildGap::Retention
        })
    )));
    let repeated = feed(
        &mut children,
        "item/completed",
        spawn("root", "child", Some("pendingInit")),
    );
    assert!(!repeated.iter().any(|event| matches!(
        event,
        AgentEvent::ProviderChild(ProviderObservation::Activity { .. })
    )));
    assert_eq!(children.entries.len(), 1);
}
#[test]
fn route_saturation_is_partial_without_evicting_known_lifecycle() {
    let mut children = Children::new(false);
    for n in 0..=vessel_proto::subagents::bounds::MAX_CHILDREN {
        let _observed = feed(
            &mut children,
            "item/completed",
            spawn("root", &format!("child-{n}"), Some("running")),
        );
    }
    assert_eq!(
        children.entries.len(),
        vessel_proto::subagents::bounds::MAX_CHILDREN
    );
    assert!(children.reported_gaps.contains(&ChildGap::IngestionBudget));
    let _observed = feed(
        &mut children,
        "item/completed",
        spawn("root", "child-0", Some("completed")),
    );
    assert_eq!(
        children.entries[&"child-0".parse::<ProviderChildId>().unwrap()]
            .descriptor
            .status,
        Some(ChildStatus::Completed)
    );
}
