//! Native child envelope fields; canonical message/tool decoding stays in protocol/mapper.
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ItemEnvelope {
    pub thread_id: String,
    pub item: ChildItem,
}
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(super) enum ChildItem {
    CollabAgentToolCall(Collab),
    SubAgentActivity(Activity),
    #[serde(other)]
    Other,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Collab {
    pub id: String,
    pub tool: Tool,
    #[serde(default)]
    pub sender_thread_id: Option<String>,
    #[serde(default)]
    pub receiver_thread_ids: Vec<String>,
    #[serde(default)]
    pub agents_states: BTreeMap<String, State>,
    #[serde(default)]
    pub prompt: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum Tool {
    SpawnAgent,
    SendInput,
    ResumeAgent,
    Wait,
    CloseAgent,
    #[serde(other)]
    Unknown,
}
#[derive(Debug, Deserialize)]
pub(super) struct State {
    pub status: Status,
    #[serde(default)]
    pub message: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum Status {
    PendingInit,
    Running,
    Completed,
    Errored,
    Shutdown,
    NotFound,
    #[serde(other)]
    Unknown,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Activity {
    pub id: String,
    pub agent_thread_id: String,
    pub agent_path: String,
    pub kind: ActivityKind,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum ActivityKind {
    Started,
    Interacted,
    Interrupted,
    #[serde(other)]
    Unknown,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ThreadEnvelope {
    pub thread_id: String,
}
