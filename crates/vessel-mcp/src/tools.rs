//! Agent MCP schemas, argument decoding and typed node-call adapters.
//!
//! [Decision Log]
//! - The existing authenticated client and destination guard enforce grants
//!   and session ownership; routing never supplies a second caller identity.
//! - `machine` and the compatibility spelling `node` choose a fleet ID or
//!   unique name. Conflicting selectors and selectors on local-only tools
//!   are refused before mutation. Names that identify several nodes require
//!   an explicit ID rather than a guessed destination.
//! - A frozen exact-provider policy controls listing and direct invocation
//!   through the shared `ToolId`. Browser identities delegate to their owner's
//!   catalogue; availability still needs the browser producer's admission.
//! - Session agents receive finish/permission/error notifications and do not
//!   advertise a wait tool. Direct legacy `wait_turn` and readiness reads
//!   retain bounded long polls for compatibility. Remote notifications follow
//!   a principal-bound exact-turn receipt at the origin; they do not infer
//!   a completed turn from the next event after a prompt was queued.

mod args;
mod automation;
mod conversation;
mod discovery;
mod driving;
mod exposure;
mod forge;
mod forking;
mod id;
mod launch;
mod lsp;
mod offer;
mod permissions;
mod response;
mod search;
mod sessions;
mod terminals;
mod waiting;
mod workspace;

use std::sync::Arc;
use std::time::Duration;

use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock as McpContent,
    Implementation, JsonObject, ListToolsResult, PaginatedRequestParams, ServerCapabilities,
    ServerConfig, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};
use serde_json::{Value, json};
use vessel_client::{CallError, NodeClient, Unary};
use vessel_proto::{FleetNode, KillSession, ListNodes, NodeId, SessionId};

use args::{flag, number, required, session, text};
use sessions::session_tools;
use waiting::after;

/// How long `wait_turn` waits when not told.
const WAIT_DEFAULT: Duration = Duration::from_secs(600);
/// The longest `wait_turn` waits.
const WAIT_MAX_SECS: u64 = 3600;
/// How long a new structured session may take to be ready for its first
/// prompt (the node's own handshake limit is 60 s by default).
const READY_TIMEOUT: Duration = Duration::from_secs(120);
/// Records `read_events` returns when not told, and at most.
const READ_DEFAULT: u32 = 100;
const READ_MAX: u32 = 1000;
/// One long poll's wait: under common 60 s idle limits on every hop.
const POLL_SECS: u32 = 30;
/// Records one poll reads.
const POLL_PAGE: u32 = 200;
/// The pause after a wait that came back empty before it could have ended
/// (the node had no room for it to wait).
const EARLY_PAUSE: Duration = Duration::from_secs(1);

/// The `vessel` tools over one connection to the agent's node.
#[derive(Debug, Clone)]
pub struct VesselTools {
    client: Arc<NodeClient>,
    /// The session whose agent this is, when known: where
    /// `create_terminal` opens one by default, and a heartbeat's target
    /// (protocol 11).
    own: Option<SessionId>,
    policy: vessel_proto::mcp::ToolPolicy,
    voice: vessel_proto::mcp::VoiceTools,
}

/// Where a tool's calls go: this node (`None`) or another of the fleet.
type Target = Option<NodeId>;

impl VesselTools {
    /// Tools calling through `client` (connected as the session's agent).
    #[must_use]
    pub fn new(client: NodeClient) -> Self {
        Self {
            client: Arc::new(client),
            own: None,
            policy: vessel_proto::mcp::ToolPolicy::default(),
            voice: vessel_proto::mcp::VoiceTools::Disabled,
        }
    }

    /// The same tools, knowing the agent's own session (`vessel mcp` reads
    /// it from its environment; `create_heartbeat` prompts it).
    #[must_use]
    pub const fn with_own_session(mut self, session: SessionId) -> Self {
        self.own = Some(session);
        self
    }

    /// The agent's own session, if known.
    const fn own_session(&self) -> Option<SessionId> {
        self.own
    }

    async fn call(&self, name: &str, args: &JsonObject) -> Result<Value, ToolError> {
        match id::decode(name) {
            id::Decoded::Tool(tool) => {
                if !self.policy.permits(tool) {
                    return Err(ToolError::Disabled { tool });
                }
                exposure::validate_selector(tool, args)?;
                self.dispatch(tool, args).await
            }
            id::Decoded::LspPrefix => self.lsp(name, args).await,
            id::Decoded::Unknown => Err(ToolError::Arguments(format!("no tool `{name}`"))),
        }
    }

    /// Run a tool whose name was already decoded.
    async fn dispatch(&self, tool: id::ToolId, args: &JsonObject) -> Result<Value, ToolError> {
        let name = tool.as_str();
        match tool {
            id::ToolId::Browser(_) => Err(ToolError::BrowserUnavailable),
            id::ToolId::LspWorkspaceSymbols
            | id::ToolId::LspDocumentSymbols
            | id::ToolId::LspDefinition
            | id::ToolId::LspReferences
            | id::ToolId::LspHover
            | id::ToolId::LspImplementations
            | id::ToolId::LspTypeDefinition
            | id::ToolId::LspIncomingCalls
            | id::ToolId::LspOutgoingCalls
            | id::ToolId::LspDiagnostics => self.lsp(name, args).await,
            id::ToolId::ListNodes => Ok(json!(self.client.call(ListNodes).await?)),
            id::ToolId::ListSessions => self.list_sessions(args).await,
            id::ToolId::ListProjects | id::ToolId::ListWorktrees | id::ToolId::CreateWorktree => {
                self.workspace(name, args).await
            }
            id::ToolId::GetPullRequest | id::ToolId::OpenPullRequest => {
                self.forge(name, args).await
            }
            id::ToolId::SearchSessions => self.search(args).await,
            id::ToolId::ForkSession | id::ToolId::RewindSession => self.forking(name, args).await,
            id::ToolId::CreateSession => self.create_session(args).await,
            id::ToolId::SendInput => self.send_input(args).await,
            id::ToolId::AnswerPermission => self.answer_permission(args).await,
            id::ToolId::ReadEvents => self.read_events(args).await,
            id::ToolId::WaitTurn => self.wait_turn(args).await,
            id::ToolId::ReadConversation
            | id::ToolId::ChangeQueue
            | id::ToolId::AnswerQuestion
            | id::ToolId::ResumeSession
            | id::ToolId::ArchiveSession => self.conversation(name, args).await,
            id::ToolId::CreateTerminal
            | id::ToolId::ListTerminals
            | id::ToolId::ReadTerminal
            | id::ToolId::WriteTerminal
            | id::ToolId::CloseTerminal
            | id::ToolId::ListPorts => self.terminals(name, args).await,
            id::ToolId::CreateSchedule
            | id::ToolId::CreateHeartbeat
            | id::ToolId::ListSchedules
            | id::ToolId::ScheduleRuns
            | id::ToolId::PauseSchedule
            | id::ToolId::ResumeSchedule
            | id::ToolId::DeleteSchedule
            | id::ToolId::ListSavedPrompts => self.automation(name, args).await,
            id::ToolId::KillSession => {
                let node = self.target(args).await?;
                let session = session(args)?;
                let _ack = self.on(node, KillSession { session }).await?;
                Ok(json!({ "killed": session }))
            }
            id::ToolId::Added(tool) => self.added(tool, args).await,
        }
    }

    /// Encode a parity tool's protocol value as the MCP result.
    async fn added(&self, tool: id::AddedId, args: &JsonObject) -> Result<Value, ToolError> {
        match tool {
            id::AddedId::GetSession => Ok(json!(self.get_session(args).await?)),
            id::AddedId::CancelTurn => {
                let cancelled = self.cancel_turn(args).await?;
                Ok(match cancelled.turn {
                    Some(turn) => json!({ "cancelled": cancelled.cancelled, "turn": turn }),
                    None => json!({ "cancelled": cancelled.cancelled }),
                })
            }
            id::AddedId::RenameSession => Ok(json!(self.rename_session(args).await?)),
            id::AddedId::SetSessionOption => Ok(json!(self.set_session_option(args).await?)),
            id::AddedId::GetSchedule => Ok(json!(self.get_schedule(args).await?)),
            id::AddedId::UpdateSchedule => Ok(json!(self.update_schedule(args).await?)),
            id::AddedId::ListHarnesses => Ok(json!(self.list_harnesses(args).await?)),
            id::AddedId::ListProfiles => Ok(json!(self.list_profiles(args).await?)),
            id::AddedId::HarnessOptions => Ok(json!(self.harness_options(args).await?)),
            id::AddedId::ListPendingPermissions => Ok(json!(self.pending_permissions(args).await?)),
        }
    }

    /// The node a `node` argument names: this one when it is left out or
    /// names this node; another by its id, or by its name in the fleet.
    async fn target(&self, args: &JsonObject) -> Result<Target, ToolError> {
        let Some(given) = exposure::selector(args)? else {
            return Ok(None);
        };
        let this = self.client.node();
        if let Ok(id) = given.parse::<NodeId>() {
            return Ok((id != this.id).then_some(id));
        }
        let nodes = self.client.call(ListNodes).await?;
        resolve(given, this.id, &this.name, &nodes)
    }

    /// `request` on `node`: here, or forwarded there.
    async fn on<R: Unary>(&self, node: Target, request: R) -> Result<R::Response, ToolError> {
        Ok(match node {
            None => self.client.call(request).await?,
            Some(node) => self.client.call_on(node, request).await?,
        })
    }
}

/// The node named `given` (not an id): this one (`this`, named
/// `this_name`) or another of `nodes` (`fleet.nodes`). A name more than
/// one node has is refused with their ids, never guessed.
fn resolve(
    given: &str,
    this: NodeId,
    this_name: &str,
    nodes: &[FleetNode],
) -> Result<Target, ToolError> {
    let mut named: Vec<NodeId> = nodes
        .iter()
        .filter(|node| node.name == given)
        .map(|node| node.node)
        .collect();
    if this_name == given && !named.contains(&this) {
        named.push(this);
    }
    match named.as_slice() {
        [] => Err(ToolError::Arguments(format!(
            "no node `{given}` in this fleet (`list_nodes` shows the ones there are)"
        ))),
        [only] => Ok((*only != this).then_some(*only)),
        [_, _, ..] => {
            let ids: Vec<String> = named.iter().map(ToString::to_string).collect();
            Err(ToolError::Arguments(format!(
                "more than one node is named `{given}` ({}); give the id instead",
                ids.join(", ")
            )))
        }
    }
}

impl ServerHandler for VesselTools {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("vessel", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Vessel: start and drive other agent sessions, on this machine or another node \
                 of its fleet (`node`, from `list_nodes`). A session you start is yours: you \
                 may send it input (queued or steered while it works), answer it, stop it, \
                 archive it and start it again. A structured session agent receives completion \
                 notifications by default; finish your turn to await the notice. To give one a folder and \
                 branch of its own, `create_worktree` in a project from `list_projects`, then \
                 `create_session` with that `worktree`. Where a person allowed it, open a \
                 terminal beside your session (`create_terminal`) and type into it.",
            )
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let mut tools = exposure::visible(&self.policy);
        if self.own_session().is_some() {
            tools.retain(|tool| tool.name != "wait_turn");
        }
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let protocol = context.protocol_version();
        let args = request.arguments.unwrap_or_default();
        match self.call(&request.name, &args).await {
            Ok(value) => Ok(response::success(value, protocol.as_ref()).into()),
            Err(ToolError::Arguments(message)) => Err(ErrorData::invalid_params(message, None)),
            Err(ToolError::LspJson(error)) => {
                Err(ErrorData::invalid_params(error.to_string(), None))
            }
            Err(refused) => {
                Ok(CallToolResult::error(vec![McpContent::text(refused.to_string())]).into())
            }
        }
    }
}

/// Why a tool call failed.
#[derive(Debug, thiserror::Error)]
enum ToolError {
    /// A shared browser identity is not a qualified engine or session admission.
    #[error("browser tools require a qualified, enabled browser engine")]
    BrowserUnavailable,
    /// The call itself is wrong: a JSON-RPC error.
    #[error("{0}")]
    Arguments(String),
    /// A direct invocation cannot bypass the launch visibility snapshot.
    #[error("tool `{}` is disabled for this session", tool.as_str())]
    Disabled { tool: id::ToolId },
    /// Typed LSP arguments or results failed to encode/decode at the SDK edge.
    #[error("invalid LSP tool JSON")]
    LspJson(#[source] serde_json::Error),
    /// The node refused or failed; the agent reads why.
    #[error("{}", node_error(.0))]
    Node(#[from] CallError),
    /// The tool could not do what it was asked.
    #[error("{0}")]
    Failed(String),
    /// A partial permission inventory must not look complete.
    #[error("permission inventory exceeds its bounded session/request limit")]
    PermissionInventory,
}

/// `code: message` for a refusal, the error's text otherwise.
fn node_error(error: &CallError) -> String {
    match error {
        CallError::Remote(error) => format!("{}: {}", json!(error.code), error.message),
        other @ (CallError::Disconnected
        | CallError::UnexpectedReply(_)
        | CallError::RequestIdsExhausted
        | CallError::NotInProtocol { .. }
        | CallError::UnassignedProtocol { .. }) => other.to_string(),
    }
}

/// Every tool with its input schema.
fn tools() -> Vec<Tool> {
    let session = json!({ "type": "string", "description": "A session id." });
    let node = node_schema();
    session_tools(&node, &session)
        .into_iter()
        .chain(driving::tools(&node, &session))
        .chain(conversation::tools(&node, &session))
        .chain(search::tools(&node, &session))
        .chain(forking::tools(&node, &session))
        .chain(workspace::tools(&node))
        .chain(forge::tools(&node, &session))
        .chain(terminals::tools(&node))
        .chain(automation::tools())
        .chain(offer::tools(&node))
        .chain(permissions::tools(&node))
        .chain(lsp::tools())
        .collect()
}

fn node_schema() -> Value {
    json!({
        "type": "string",
        "description": "A node of the fleet, by its id or its name in `list_nodes`; leave it \
                        out for this node."
    })
}

fn tool(
    name: &'static str,
    description: &'static str,
    mut properties: Value,
    required: &[&str],
) -> Tool {
    if let Some(fields) = properties.as_object_mut()
        && let Some(node) = fields.get("node").cloned()
    {
        let _old = fields.insert("machine".to_owned(), node);
    }
    let mut schema = JsonObject::new();
    let _none = schema.insert("type".to_owned(), json!("object"));
    let _none = schema.insert("properties".to_owned(), properties);
    if !required.is_empty() {
        let _none = schema.insert("required".to_owned(), json!(required));
    }
    Tool::new(name, description, schema)
}

#[cfg(test)]
mod tests {
    use vessel_proto::PROTOCOL_VERSION;

    use super::*;

    fn node(id: NodeId, name: &str) -> FleetNode {
        FleetNode {
            node: id,
            name: name.to_owned(),
            protocol: PROTOCOL_VERSION,
            reachable: true,
        }
    }

    /// Names are each node's own choice: one that more than one node has is
    /// refused, whichever they are (the review of Track X, item 5).
    #[test]
    fn a_name_names_one_node_or_none() {
        let (here, desktop, laptop) = (NodeId::random(), NodeId::random(), NodeId::random());
        let fleet = [
            node(here, "home"),
            node(desktop, "desktop"),
            node(laptop, "laptop"),
        ];
        assert_eq!(resolve("home", here, "home", &fleet).unwrap(), None);
        assert_eq!(
            resolve("desktop", here, "home", &fleet).unwrap(),
            Some(desktop)
        );
        assert!(matches!(
            resolve("tablet", here, "home", &fleet),
            Err(ToolError::Arguments(_))
        ));
        // Outside a fleet `fleet.nodes` lists this node alone.
        assert_eq!(
            resolve("home", here, "home", &[node(here, "home")]).unwrap(),
            None
        );

        // Two members of one name, and another node named like this one.
        let twins = [
            node(here, "home"),
            node(desktop, "desktop"),
            node(laptop, "desktop"),
        ];
        let Err(ToolError::Arguments(message)) = resolve("desktop", here, "home", &twins) else {
            panic!("two nodes named alike were not refused");
        };
        assert!(
            message.contains(&desktop.to_string()) && message.contains(&laptop.to_string()),
            "{message}"
        );
        let mimic = [node(here, "home"), node(desktop, "home")];
        assert!(matches!(
            resolve("home", here, "home", &mimic),
            Err(ToolError::Arguments(_))
        ));
    }
}
