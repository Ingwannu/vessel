//! Known `TodoWrite` arguments become typed task snapshots; labels do not select it.
use serde::Deserialize;
use serde_json::Value;
use vessel_proto::timeline::TimelineTaskId;
use vessel_proto::{AgentEvent, PlanEntry, PlanPriority, PlanStatus};

const MAX_CANONICAL_TASKS: usize = 1024;
#[derive(Deserialize)]
struct TodoWrite {
    todos: Vec<Todo>,
}
#[derive(Deserialize)]
struct Todo {
    content: String,
    status: PlanStatus,
    #[serde(default)]
    id: Option<TimelineTaskId>,
    #[serde(default, rename = "activeForm")]
    active_form: Option<String>,
}

pub(crate) fn tasks(name: &str, input: &Value) -> Option<AgentEvent> {
    if !matches!(name, "TodoWrite" | "todo_write") {
        return None;
    }
    let input: TodoWrite = serde_json::from_value(input.clone()).ok()?;
    if input.todos.len() > MAX_CANONICAL_TASKS {
        return None;
    }
    Some(AgentEvent::Plan {
        entries: input
            .todos
            .into_iter()
            .map(|todo| PlanEntry {
                content: todo.content,
                status: todo.status,
                id: todo.id,
                active_form: todo.active_form,
                priority: PlanPriority::Medium,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn task_identity_and_running_label_survive_rename_reorder_and_empty_snapshot() {
        let AgentEvent::Plan { entries } = tasks("TodoWrite", &json!({"todos":[{"id":"a","content":"renamed","activeForm":"Working","status":"in_progress"},{"id":"b","content":"Done","status":"completed"}]})).unwrap() else { panic!("plan") };
        assert_eq!(entries[0].id.as_ref().unwrap().as_str(), "a");
        assert_eq!(entries[0].active_form.as_deref(), Some("Working"));
        assert_eq!(entries[0].content, "renamed");
        assert_eq!(entries[1].status, PlanStatus::Completed);
        assert_eq!(
            tasks("todo_write", &json!({"todos":[]})),
            Some(AgentEvent::Plan { entries: vec![] })
        );
        assert!(tasks("ExitPlanMode", &json!({"todos":[]})).is_none());
        assert!(
            tasks(
                "TodoWrite",
                &json!({"todos":[{"content":"bad","status":"future"}]})
            )
            .is_none()
        );
        assert!(tasks("TodoWrite", &json!({"todos":vec![json!({"content":"task","status":"pending"});MAX_CANONICAL_TASKS+1]})).is_none());
    }
}
