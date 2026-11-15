//! The part of Codex's app-server protocol the driver speaks, as types.
//!
//! Each type names the definition it follows in `schema/app-server.json`,
//! the subset of `codex app-server generate-json-schema --experimental`
//! (Codex 0.166.0) the driver uses; `contract` checks that every field
//! read or written here is in that file, and that the file still matches
//! an installed Codex.
//!
//! [Decision Log]
//! - Purpose: the driver reads the agent's messages as typed values, so a
//!   renamed field fails a test rather than reading as absent.
//! - Choice: only the fields the driver uses, each read leniently (unknown
//!   fields ignored, unknown item and status kinds kept as `Other`):
//!   Codex adds fields and kinds at most releases, and a session must not
//!   fail on one it does not show.
//! - Alternatives: types generated from the whole schema (some 400
//!   definitions, most of which the driver never touches, regenerated at
//!   each Codex release); `serde_json::Value` throughout (no test catches
//!   a rename).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `InitializeParams`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct InitializeParams {
    pub(super) client_info: ClientInfo,
    pub(super) capabilities: InitializeCapabilities,
}

/// `ClientInfo`.
#[derive(Debug, Serialize)]
pub(super) struct ClientInfo {
    pub(super) name: &'static str,
    pub(super) title: &'static str,
    pub(super) version: &'static str,
}

/// `InitializeCapabilities`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct InitializeCapabilities {
    /// The experimental fields (Daybreak's `cyberAccessProgram`, plan
    /// mode) are offered only with it.
    pub(super) experimental_api: bool,
}

/// `InitializeResponse`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct InitializeResponse {
    #[serde(default)]
    pub(super) user_agent: Option<String>,
}

/// `AskForApproval`: when the agent asks before acting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum AskForApproval {
    /// Asks when its sandbox or its rules say a step needs it.
    OnRequest,
    /// Never asks (full access only).
    Never,
}

/// `ApprovalsReviewer`: who reviews what the agent asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ApprovalsReviewer {
    /// A person (the host).
    User,
    /// Codex's own reviewer first, a person for what it will not decide.
    AutoReview,
}

/// `SandboxMode`: a thread's sandbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum SandboxMode {
    /// Reads only.
    ReadOnly,
    /// Writes in the workspace only.
    WorkspaceWrite,
    /// No sandbox (full access only).
    DangerFullAccess,
}

/// `SandboxPolicy`: a turn's sandbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum SandboxPolicy {
    ReadOnly { network_access: bool },
    WorkspaceWrite { network_access: bool },
    DangerFullAccess,
}

/// `ThreadStartParams`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ThreadStartParams {
    pub(super) cwd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) model: Option<String>,
    pub(super) approval_policy: AskForApproval,
    pub(super) approvals_reviewer: ApprovalsReviewer,
    pub(super) sandbox: SandboxMode,
}

/// `ThreadResumeParams`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ThreadResumeParams {
    pub(super) thread_id: String,
    pub(super) cwd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) model: Option<String>,
    pub(super) approval_policy: AskForApproval,
    pub(super) approvals_reviewer: ApprovalsReviewer,
    pub(super) sandbox: SandboxMode,
    /// The history is read where it is replayed (thread/turns/list), never
    /// in this one answer.
    pub(super) exclude_turns: bool,
}

/// `ThreadStartResponse` and `ThreadResumeResponse`: the thread.
#[derive(Debug, Deserialize)]
pub(super) struct ThreadOpened {
    pub(super) thread: Thread,
}

/// `Thread`.
#[derive(Debug, Deserialize)]
pub(super) struct Thread {
    pub(super) id: String,
}

/// `UserInput`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(super) enum UserInput {
    /// Text.
    Text { text: String },
    /// A file or folder named in the prompt.
    Mention { name: String, path: String },
}

/// `TurnStartParams`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TurnStartParams {
    pub(super) thread_id: String,
    pub(super) input: Vec<UserInput>,
    /// The session's mode as it stands now (a change applies from the
    /// next turn).
    pub(super) approval_policy: AskForApproval,
    pub(super) approvals_reviewer: ApprovalsReviewer,
    pub(super) sandbox_policy: SandboxPolicy,
}

/// `TurnStartResponse`.
#[derive(Debug, Deserialize)]
pub(super) struct TurnStartResponse {
    pub(super) turn: Turn,
}

/// `TurnInterruptParams`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TurnInterruptParams {
    pub(super) thread_id: String,
    pub(super) turn_id: String,
}

/// `TurnSteerParams`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TurnSteerParams {
    pub(super) thread_id: String,
    pub(super) expected_turn_id: String,
    pub(super) input: Vec<UserInput>,
}

/// `Turn`.
#[derive(Debug, Clone, Deserialize)]
pub(super) struct Turn {
    pub(super) id: String,
    pub(super) status: TurnStatus,
    #[serde(default)]
    pub(super) error: Option<TurnError>,
}

/// `TurnStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum TurnStatus {
    Completed,
    Interrupted,
    Failed,
    InProgress,
    #[serde(other)]
    Other,
}

/// `TurnError`.
#[derive(Debug, Clone, Deserialize)]
pub(super) struct TurnError {
    pub(super) message: String,
}

/// `TurnStartedNotification` and `TurnCompletedNotification`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TurnNotification {
    pub(super) thread_id: String,
    pub(super) turn: Turn,
}

/// `ItemStartedNotification` and `ItemCompletedNotification`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ItemNotification {
    pub(super) thread_id: String,
    pub(super) item: ThreadItem,
}

/// `AgentMessageDeltaNotification` and
/// `ReasoningSummaryTextDeltaNotification`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DeltaNotification {
    pub(super) thread_id: String,
    pub(super) item_id: String,
    pub(super) delta: String,
}

/// `TurnPlanUpdatedNotification`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PlanNotification {
    pub(super) thread_id: String,
    pub(super) plan: Vec<TurnPlanStep>,
}

/// `TurnPlanStep`.
#[derive(Debug, Deserialize)]
pub(super) struct TurnPlanStep {
    pub(super) step: String,
    pub(super) status: TurnPlanStepStatus,
}

/// `TurnPlanStepStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum TurnPlanStepStatus {
    Pending,
    InProgress,
    Completed,
    #[serde(other)]
    Other,
}

/// `ThreadTokenUsageUpdatedNotification`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UsageNotification {
    pub(super) thread_id: String,
    pub(super) token_usage: ThreadTokenUsage,
}

/// `ThreadTokenUsage`.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ThreadTokenUsage {
    /// The thread's running total.
    pub(super) total: TokenUsageBreakdown,
    /// The last request's: what the context holds now.
    pub(super) last: TokenUsageBreakdown,
    #[serde(default)]
    pub(super) model_context_window: Option<u64>,
}

/// `TokenUsageBreakdown`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[expect(
    clippy::struct_field_names,
    reason = "Codex's own names, kept so each reads as the schema says"
)]
#[serde(rename_all = "camelCase")]
pub(super) struct TokenUsageBreakdown {
    pub(super) input_tokens: u64,
    pub(super) cached_input_tokens: u64,
    pub(super) output_tokens: u64,
    pub(super) reasoning_output_tokens: u64,
    pub(super) total_tokens: u64,
}

/// `ErrorNotification`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ErrorNotification {
    pub(super) thread_id: String,
    pub(super) error: TurnError,
    pub(super) will_retry: bool,
}

/// `ThreadItem`: the kinds the driver shows; any other is `Other`.
#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum ThreadItem {
    AgentMessage {
        id: String,
        text: String,
    },
    Reasoning {
        id: String,
        #[serde(default)]
        summary: Vec<String>,
    },
    CommandExecution {
        id: String,
        command: String,
        #[serde(default)]
        cwd: Option<String>,
        status: ItemStatus,
        #[serde(default)]
        aggregated_output: Option<String>,
        #[serde(default)]
        exit_code: Option<i64>,
    },
    FileChange {
        id: String,
        changes: Vec<FileUpdateChange>,
        status: ItemStatus,
    },
    McpToolCall {
        id: String,
        server: String,
        tool: String,
        #[serde(default)]
        arguments: Value,
        status: ItemStatus,
    },
    WebSearch {
        id: String,
        query: String,
    },
    #[serde(other)]
    Other,
}

/// `CommandExecutionStatus`, `PatchApplyStatus` and `McpToolCallStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum ItemStatus {
    InProgress,
    Completed,
    Failed,
    Declined,
    #[serde(other)]
    Other,
}

/// `FileUpdateChange`.
#[derive(Debug, Clone, Deserialize)]
pub(super) struct FileUpdateChange {
    pub(super) path: String,
    pub(super) diff: String,
}

/// `ServerRequestResolvedNotification`: Codex no longer waits on one of
/// its requests (answered elsewhere, or its turn ended).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RequestResolved {
    /// The request's id, as Codex sent it.
    pub(super) request_id: Value,
}

/// `CommandExecutionRequestApprovalParams`: what a command's approval
/// shows, and the decisions Codex offers for it.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CommandApproval {
    pub(super) item_id: String,
    #[serde(default)]
    pub(super) command: Option<String>,
    #[serde(default)]
    pub(super) cwd: Option<String>,
    /// `CommandExecutionApprovalDecision`s, each a string or an object
    /// (one carrying the rule or the network amendment to apply); absent
    /// means the four plain ones.
    #[serde(default)]
    pub(super) available_decisions: Option<Vec<Value>>,
}

/// `FileChangeRequestApprovalParams`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FileChangeApproval {
    pub(super) item_id: String,
    #[serde(default)]
    pub(super) reason: Option<String>,
}

/// `PermissionsRequestApprovalParams`: more permissions asked for.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PermissionsApproval {
    pub(super) item_id: String,
    /// `RequestPermissionProfile`, granted as asked when allowed.
    pub(super) permissions: Value,
    #[serde(default)]
    pub(super) reason: Option<String>,
}

/// `ToolRequestUserInputParams`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UserInputRequest {
    pub(super) item_id: String,
    pub(super) questions: Vec<UserInputQuestion>,
}

/// `ToolRequestUserInputQuestion`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UserInputQuestion {
    pub(super) id: String,
    pub(super) header: String,
    pub(super) question: String,
    #[serde(default)]
    pub(super) is_other: bool,
    /// Never shown: an answer is kept in the session's timeline.
    #[serde(default)]
    pub(super) is_secret: bool,
    #[serde(default)]
    pub(super) options: Option<Vec<UserInputOption>>,
}

/// `ToolRequestUserInputOption`.
#[derive(Debug, Deserialize)]
pub(super) struct UserInputOption {
    pub(super) label: String,
    pub(super) description: String,
}
