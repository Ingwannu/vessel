//! Real wire shapes in memory; no installed provider or subprocess is used.
use super::*;
use serde_json::json;
use vessel_proto::{
    ContentBlock,
    subagents::{ChildStatus, ProviderObservation, bounds},
};

fn feed(children: &mut Children, frame: serde_json::Value) -> Vec<AgentEvent> {
    children.observe(&serde_json::from_value::<Frame>(frame).unwrap())
}
fn started(task: &str, tool: &str, background: Option<bool>) -> serde_json::Value {
    json!({"type":"system","subtype":"task_started","task_id":task,"tool_use_id":tool,
        "task_type":"local_agent","subagent_type":"Explore","description":"Find configuration",
        "prompt":"Read the actual configuration","is_backgrounded":background})
}
fn descriptors(events: &[AgentEvent]) -> Vec<&ChildDescriptor> {
    events
        .iter()
        .filter_map(|event| {
            if let AgentEvent::ProviderChild(ProviderObservation::Descriptor { descriptor }) = event
            {
                Some(descriptor)
            } else {
                None
            }
        })
        .collect()
}
fn message_events(events: &[AgentEvent], id: &str) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| {
            if let AgentEvent::ProviderChild(ProviderObservation::Activity { child, activity }) =
                event
                && child.as_str() == id
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
fn assistant(parent: &str, message: &str, text: &str) -> serde_json::Value {
    json!({"type":"assistant","parent_tool_use_id":parent,"message":{"id":message,"content":[{"type":"text","text":text}]}})
}

#[test]
fn background_lifecycle_without_any_sidechain_survives_the_main_result() {
    let mut children = Children::new(false);
    let events = feed(&mut children, started("task-one", "tool-one", Some(true)));
    let descriptor = descriptors(&events)[0];
    assert_eq!(descriptor.id.as_str(), "task-one");
    assert_eq!(descriptor.status, Some(ChildStatus::Running));
    assert_eq!(descriptor.created_at, None);
    assert_eq!(descriptor.updated_at, None);
    assert!(feed(&mut children, json!({"type":"result","subtype":"success"})).is_empty());
    assert!(children.canceled().is_empty());
    let events = feed(
        &mut children,
        json!({"type":"system","subtype":"task_updated","task_id":"task-one","patch":{"status":"completed"}}),
    );
    assert_eq!(descriptors(&events)[0].status, Some(ChildStatus::Completed));
}
#[test]
fn real_task_identity_survives_reannouncements_and_restart_with_different_tool_aliases() {
    let mut first = Children::new(false);
    let _observed = feed(&mut first, started("stable-task", "old-tool", Some(true)));
    let events = feed(&mut first, started("stable-task", "new-tool", Some(true)));
    assert_eq!(first.entries.len(), 1);
    assert_eq!(descriptors(&events)[0].id.as_str(), "stable-task");
    assert_eq!(
        message_events(
            &feed(&mut first, assistant("new-tool", "message", "reply")),
            "stable-task"
        ),
        vec!["reply"]
    );
    let mut resumed = Children::new(true);
    let events = feed(
        &mut resumed,
        started("stable-task", "resumed-tool", Some(true)),
    );
    assert_eq!(descriptors(&events)[0].id.as_str(), "stable-task");
    assert!(events.iter().any(|event| matches!(
        event,
        AgentEvent::ProviderChild(ProviderObservation::Gap {
            reason: ChildGap::Retention
        })
    )));
}
#[test]
fn background_shell_and_unknown_task_kinds_are_not_fabricated_children() {
    let mut children = Children::new(false);
    for kind in ["local_bash", "future_task_kind"] {
        let _observed = feed(
            &mut children,
            json!({"type":"system","subtype":"task_started","task_id":"shell","tool_use_id":"shell-tool","task_type":kind}),
        );
        assert!(
            feed(
                &mut children,
                assistant("shell-tool", "shell-message", "shell text")
            )
            .is_empty()
        );
    }
    assert!(children.entries.is_empty());
}
#[test]
fn canonical_streaming_dedup_is_per_child_and_main_translation_still_ignores_child_frames() {
    let mut children = Children::new(false);
    let _observed = feed(&mut children, started("task-a", "tool-a", Some(false)));
    let _observed = feed(&mut children, started("task-b", "tool-b", Some(false)));
    let _observed = feed(
        &mut children,
        json!({"type":"stream_event","parent_tool_use_id":"tool-a","event":{"type":"message_start","message":{"id":"same-message"}}}),
    );
    let events = feed(
        &mut children,
        json!({"type":"stream_event","parent_tool_use_id":"tool-a","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"streamed"}}}),
    );
    assert_eq!(message_events(&events, "task-a"), vec!["streamed"]);
    assert!(
        message_events(
            &feed(
                &mut children,
                assistant("tool-a", "same-message", "streamed")
            ),
            "task-a"
        )
        .is_empty()
    );
    assert_eq!(
        message_events(
            &feed(
                &mut children,
                assistant("tool-b", "same-message", "sibling")
            ),
            "task-b"
        ),
        vec!["sibling"]
    );
    let frame: Frame =
        serde_json::from_value(assistant("tool-a", "another", "child only")).unwrap();
    if let Frame::Assistant(message) = frame {
        assert!(Translator::default().assistant(&message).is_empty());
    }
}
#[test]
fn nested_task_parent_is_observed_from_its_actual_emitting_tool() {
    let mut children = Children::new(false);
    let _observed = feed(
        &mut children,
        started("outer-task", "outer-tool", Some(false)),
    );
    let _observed = feed(
        &mut children,
        json!({"type":"assistant","parent_tool_use_id":"outer-tool","message":{"id":"outer-message","content":[{"type":"tool_use","id":"nested-tool","name":"Task","input":{"description":"nested"}}]}}),
    );
    let events = feed(
        &mut children,
        started("nested-task", "nested-tool", Some(true)),
    );
    assert_eq!(
        descriptors(&events)[0].parent.as_ref().unwrap().as_str(),
        "outer-task"
    );
}
#[test]
fn task_tool_completion_does_not_complete_a_running_background_child() {
    let mut children = Children::new(false);
    let _observed = feed(&mut children, started("task", "tool", Some(true)));
    let _observed = feed(
        &mut children,
        json!({"type":"assistant","parent_tool_use_id":"tool","message":{"id":"m","content":[{"type":"tool_use","id":"read-call","name":"Read","input":{"file_path":"/fixture"}}]}}),
    );
    let events = feed(
        &mut children,
        json!({"type":"user","parent_tool_use_id":"tool","message":{"content":[{"type":"tool_result","tool_use_id":"read-call","content":"result"}]}}),
    );
    assert!(events.iter().any(|event|matches!(event,AgentEvent::ProviderChild(ProviderObservation::Activity {activity,..}) if matches!(activity.event(),AgentEvent::ToolCallUpdate(_)))));
    assert_eq!(
        children.entries[0].descriptor.status,
        Some(ChildStatus::Running)
    );
    assert!(feed(&mut children,json!({"type":"system","subtype":"task_updated","task_id":"task","patch":{"status":"new_unknown_state"}})).is_empty());
    assert_eq!(
        children.entries[0].descriptor.status,
        Some(ChildStatus::Running)
    );
}
#[test]
fn cancellation_only_terminalizes_explicitly_foreground_tasks_and_keeps_aliases() {
    let mut children = Children::new(false);
    let _observed = feed(&mut children, started("foreground", "fg-tool", Some(false)));
    let _observed = feed(&mut children, started("background", "bg-tool", Some(true)));
    let _observed = feed(&mut children, started("unknown", "unknown-tool", None));
    let events = children.canceled();
    assert_eq!(descriptors(&events).len(), 1);
    assert_eq!(descriptors(&events)[0].id.as_str(), "foreground");
    assert_eq!(descriptors(&events)[0].status, Some(ChildStatus::Canceled));
    assert_eq!(children.aliases.len(), 3);
}
#[test]
fn descriptor_saturation_is_explicit_non_evicting_and_does_not_hide_known_terminal_updates() {
    let mut children = Children::new(false);
    let mut gaps = 0;
    for index in 0..=bounds::MAX_CHILDREN {
        let events = feed(
            &mut children,
            started(
                &format!("task-{index}"),
                &format!("tool-{index}"),
                Some(true),
            ),
        );
        gaps += events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    AgentEvent::ProviderChild(ProviderObservation::Gap {
                        reason: ChildGap::IngestionBudget
                    })
                )
            })
            .count();
    }
    assert_eq!(children.entries.len(), bounds::MAX_CHILDREN);
    assert_eq!(gaps, 1);
    assert_eq!(children.entries[0].descriptor.id.as_str(), "task-0");
    let events = feed(
        &mut children,
        json!({"type":"system","subtype":"task_notification","task_id":"task-0","status":"failed"}),
    );
    assert_eq!(descriptors(&events)[0].status, Some(ChildStatus::Failed));
}
#[test]
fn legacy_sidechains_keep_real_tool_identity_with_unknown_lifecycle() {
    let mut children = Children::new(false);
    let events = feed(
        &mut children,
        assistant("legacy-task-tool", "m", "legacy reply"),
    );
    assert_eq!(descriptors(&events)[0].id.as_str(), "legacy-task-tool");
    assert_eq!(descriptors(&events)[0].status, None);
    assert_eq!(
        message_events(&events, "legacy-task-tool"),
        vec!["legacy reply"]
    );
}

#[test]
fn a_declared_background_task_without_a_tool_alias_keeps_real_identity_and_honest_coverage() {
    let mut children = Children::new(false);
    let mut task = started("background-task", "absent", Some(true));
    let _alias = task.as_object_mut().unwrap().remove("tool_use_id");
    let events = feed(&mut children, task);
    assert_eq!(descriptors(&events)[0].id.as_str(), "background-task");
    assert_eq!(descriptors(&events)[0].status, Some(ChildStatus::Running));
    assert!(children.aliases.is_empty());
    assert!(events.iter().any(|event| matches!(
        event,
        AgentEvent::ProviderChild(ProviderObservation::Gap {
            reason: ChildGap::Retention
        })
    )));
    let events = feed(
        &mut children,
        json!({"type":"system","subtype":"task_notification","task_id":"background-task","status":"completed"}),
    );
    assert_eq!(descriptors(&events)[0].status, Some(ChildStatus::Completed));
}
