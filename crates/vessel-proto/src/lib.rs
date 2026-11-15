//! Wire contract between Vessel nodes and their clients.
//!
//! Everything a node, the CLI, a web UI or an editor plugin needs to agree
//! on: identifiers, harnesses, sessions, the session event log, the
//! structured agent timeline, errors, and the typed RPC frames.
//!
//! Rules this crate keeps (see `AGENTS.md`):
//! - no I/O and no async runtime, so any client can depend on it;
//! - no untyped JSON: open-ended payloads are [`RawJson`], never a `Value`;
//! - it is the only place wire types are defined;
//! - documented local client-book values also live here so client and view
//!   share types without an I/O/UI edge. They add no RPC/version/TS field.

mod agent;
pub mod agents;
pub mod automation;
mod base64_bytes;
mod board;
mod book_route;
pub mod book_values;
pub mod conversation;
pub mod creation;
mod device;
pub mod diagnostics;
pub mod dictation;
mod error;
mod event;
mod fleet;
mod fleet_control;
mod fleet_published;
pub mod forge;
pub mod forking;
mod forward;
mod harness;
pub mod history;
mod id;
pub mod inbox;
pub mod inbox_attention;
mod key;
pub mod labels;
pub mod lifecycle_version;
pub mod limits;
pub mod lsp;
mod model;
mod name;
mod node;
pub mod notifications;
mod options;
mod pairing;
pub mod parking;
pub mod plugins;
pub mod ports;
mod profile;
mod proxy;
pub mod push;
mod raw;
pub mod registry;
mod relay;
pub mod restart;
mod rpc;
pub mod scripts;
pub mod search;
mod secret;
mod session;
pub mod settings;
pub mod side_questions;
pub mod subagents;
mod terminal;
pub mod terminal_profiles;
pub mod terminals;
pub mod timeline;
mod title;
mod transfer;
#[cfg(feature = "ts")]
pub mod ts;
pub mod usage;
pub mod workspace;

pub use agent::{
    AgentEvent, ContentBlock, PermissionOption, PermissionOptionKind, PermissionOutcome,
    PermissionRequest, PlanEntry, PlanPriority, PlanStatus, StopReason, ToolCall, ToolCallContent,
    ToolCallStatus, ToolCallUpdate, ToolKind, ToolLocation, TurnPoint,
};
pub use agents::{AgentName, AgentNameError, HarnessRef, TerminalSupport, UnknownAgent};
pub use board::{Activity, DisplayStatus, KanbanColumn, SessionBoard};
pub use book_route::BookRouteId;
pub use conversation::{
    AgentAbilities, AgentCommand, AttachmentKind, AttachmentLimits, AttachmentName, AttachmentRef,
    ContextUsage, Conversation, Delivered, Delivery, KeptMarks, Mark, PromptQueue, Question,
    QuestionAnswer, QuestionOutcome, QueueChange, QueueHold, QueuedPrompt, SessionMarks, TurnUsage,
};
pub use device::{
    DeviceInfo, DeviceInviteError, DeviceSecret, DeviceSelf, GetDeviceSelf, Invitation,
    InvitationSecret, InviteDevice, ListDevices, Preamble, PreambleReply, RefusalReason,
    RevokeDevice, Scope,
};
pub use error::{ErrorCode, ErrorDetail, WireError};
pub use event::{EventRecord, SessionEvent};
pub use fleet::{
    BlobChunk, BlobHash, BundleFile, BundlePath, BundlePathRule, ConfigValue, FleetMember,
    Generation, GetManifest, HttpUrl, InvalidBundlePath, InvalidHttpUrl, ListMembers, Manifest,
    McpDefinitions, McpServers, McpTransport, ReadBlob, ReportApplied, ResolvedMcpServers,
    ResolvedValue, WatchFleet,
};
pub use fleet_control::{
    FleetApplied, FleetControlError, FleetRole, FleetSyncProblem, FleetSyncStatus, GetFleetStatus,
    JoinFleet, JoinedFleet, LeaveFleet, SyncFleet,
};
pub use fleet_published::{
    FleetPublished, GetPublished, PublishedBundle, SharedFile, SharedFileKind,
};
pub use forward::{
    Callback, Deliver, FleetNode, ForgetNode, Forward, Forwarded, GetCallback, GrantHome,
    ListNodes, ListPeers, NodeForgotten, Origin, OriginBy, RelayCall,
};
pub use harness::{Harness, HarnessKind, StructuredSupport, UnknownHarness};
pub use id::{
    AgentMessageId, AttachmentId, DeviceId, EventSeq, HarnessSessionId, HookId, InstallId, ModelId,
    NodeId, OptionId, OptionValueId, PermissionId, PermissionOptionId, ProjectId, QuestionId,
    QueueEntryId, RequestId, SavedPromptId, ScheduleId, SessionId, SubscriptionId, TerminalId,
    ToolCallId, TransferId, UploadId, WorkspaceId, WorkspaceOperationId, WorktreeId,
};
pub use id::{SetupRevision, SetupRunId};
pub use key::{KeyTextError, NodePublicKey, RoutingId};
pub use labels::{InvalidLabel, LabelKey, LabelValue, SessionLabels};
pub use model::{
    DurationMs, ListModels, ModelApi, ModelCall, ModelFailure, ModelRoute, ModelStop, SessionModel,
    TokenCount, TokenUsage,
};
pub use name::{
    EnvName, HeaderName, InvalidName, McpServerName, ModelRouteName, NameKind, ProfileName,
    ProviderName, SecretName,
};
pub use node::{
    GENERATED_NAMES, MIN_PROTOCOL_VERSION, NodeInfo, NodeState, PROTOCOL_VERSION, ProtocolRange,
    ProtocolVersion, speaks_generated_names,
};
pub use options::{
    ChoiceValue, GetHarnessOptions, GetSessionOptions, HarnessOptions, OptionCategory,
    OptionChoice, OptionKind, OptionValue, SessionOption, SetSessionOption,
};
pub use pairing::{InvalidRoute, LinkError, OfferVersion, PairingLink, PairingOffer, RouteUrl};
pub use profile::{
    AgentAuth, AgentSecret, Creator, ListProfiles, MoveProfile, ProfileInfo, ReloadProfiles,
};
pub use proxy::ProxyExit;
pub use raw::RawJson;
pub use relay::{
    ConnId, EmptyToken, HandshakeMessage, NodeClose, NodeToRelay, RegistrationRefusal, RelayClose,
    RelayTicket, RelayToNode, RelayToken, TicketMac,
};
pub use rpc::{
    Ack, Call, ClientFrame, ClientInfo, CreateSession, EventPage, GetNode, GetSession, Handler,
    Hello, KillSession, ListHarnesses, ListSessions, Method, MethodShape, Outcome, ReadEvents,
    RenameSession, Reply, Request, SendInput, ServerFrame, StopNode, StopWhen, Subscribe,
    Subscribed, SubscriptionEnd, UnexpectedReply, Unsubscribe, Welcome, dispatch,
};
pub use secret::{CopySecret, DeleteSecret, ListSecrets, SecretValue, SetSecret};
pub use session::{
    RemoteCreator, SessionInfo, SessionInput, SessionMode, SessionOrigin, SessionSpec,
    SessionStatus,
};
pub use terminal::{TerminalBytes, TerminalInput, TerminalOutput, TerminalSize};
pub use title::{
    GeneratedTitle, InvalidGeneratedTitle, InvalidTitle, MAX_GENERATED_TITLE_UNITS, SessionTitle,
    TitleOrigin,
};
pub use transfer::{
    CloseTransfer, ExportMemory, ExportSession, ImportMemory, ImportSession, MAX_TRANSFER_FILE,
    MAX_TRANSFER_TOTAL, MemoryImported, MemoryRef, ReadTransfer, SessionImported, SessionRef,
    TransferDetail, TransferManifest,
};

/// Default TCP port a node listens on.
pub const DEFAULT_PORT: u16 = 7077;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn session_spec_flattens_mode() {
        let spec = SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: Some(ModelId::new("opus")),
                resume: None,
                options: Vec::new(),
            },
        )
        .with_cwd(PathBuf::from("/work"));
        let json = serde_json::to_string(&spec).unwrap();
        assert_eq!(
            json,
            r#"{"harness":"claude-code","cwd":"/work","mode":"structured","model":"opus"}"#
        );
        assert_eq!(serde_json::from_str::<SessionSpec>(&json).unwrap(), spec);
    }

    #[test]
    fn terminal_mode_defaults_apply() {
        let spec: SessionSpec =
            serde_json::from_str(r#"{"harness":"codex","mode":"terminal"}"#).unwrap();
        assert_eq!(
            spec.mode,
            SessionMode::Terminal {
                args: Vec::new(),
                size: TerminalSize::default(),
            }
        );
    }

    #[test]
    fn event_records_round_trip() {
        let record = EventRecord {
            seq: EventSeq::new(3),
            at: chrono::DateTime::UNIX_EPOCH,
            event: SessionEvent::Agent {
                event: AgentEvent::ToolCall(ToolCall {
                    id: ToolCallId::new("call_1"),
                    title: "Run tests".to_owned(),
                    kind: ToolKind::Execute,
                    status: ToolCallStatus::Pending,
                    content: Vec::new(),
                    locations: Vec::new(),
                    raw_input: Some(RawJson::from_text(r#"{"cmd":"cargo test"}"#).unwrap()),
                }),
            },
        };
        let json = serde_json::to_string(&record).unwrap();
        assert_eq!(serde_json::from_str::<EventRecord>(&json).unwrap(), record);
    }
}
