//! Actual Claude task lifecycle frames, distinct from background shell tasks.
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskKind {
    LocalAgent,
    LocalWorkflow,
    LocalBash,
    #[serde(other)]
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Killed,
    Stopped,
    #[serde(other)]
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct TaskStarted {
    pub(crate) task_id: String,
    #[serde(default)]
    pub(crate) tool_use_id: Option<String>,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) subagent_type: Option<String>,
    #[serde(default)]
    pub(crate) task_type: Option<TaskKind>,
    #[serde(default)]
    pub(crate) prompt: Option<String>,
    #[serde(default)]
    pub(crate) skip_transcript: bool,
    #[serde(default)]
    pub(crate) is_backgrounded: Option<bool>,
}
impl TaskStarted {
    pub(crate) fn is_child(&self) -> bool {
        !self.skip_transcript
            && match self.task_type {
                Some(TaskKind::LocalAgent | TaskKind::LocalWorkflow) => true,
                Some(TaskKind::LocalBash | TaskKind::Unknown) => false,
                None => self
                    .subagent_type
                    .as_deref()
                    .is_some_and(|kind| !kind.trim().is_empty()),
            }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct TaskPatch {
    #[serde(default)]
    pub(crate) status: Option<TaskStatus>,
    #[serde(default)]
    pub(crate) is_backgrounded: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct TaskUpdated {
    pub(crate) task_id: String,
    #[serde(default)]
    pub(crate) patch: Option<TaskPatch>,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct TaskUsage {
    #[serde(default)]
    pub(crate) total_tokens: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct TaskNotification {
    pub(crate) task_id: String,
    #[serde(default)]
    pub(crate) status: Option<TaskStatus>,
    #[serde(default)]
    pub(crate) usage: Option<TaskUsage>,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct TaskProgress {
    pub(crate) task_id: String,
    #[serde(default)]
    pub(crate) usage: Option<TaskUsage>,
}
