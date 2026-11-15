//! The tools, each one protocol call (or a call and a wait) through the
//! agent's own principal. Arguments and results are JSON; sessions, events
//! and nodes appear as the protocol describes them (PROTOCOL.md).
//!
//! [Decision Log]
//! - Other nodes (protocol 6, Track X): every tool but `list_nodes` takes
//!   `node`, a node's id or its name in `list_nodes` (`fleet.nodes`); a
//!   call for another node goes in `fleet.forward` (`NodeClient::call_on`),
//!   and that node applies its own grant, ownership and limits to the
//!   agent. Left out or naming this node, it is a call here. Names are
//!   chosen by each node, so a name more than one node has (this one and
//!   another included) is refused with the ids to choose from, never
//!   guessed (the review of Track X, item 5; the home also refuses a
//!   member named like another).
//! - Waiting is a long poll, here and elsewhere alike: `wait_turn` (and a
//!   new session's wait to be ready) is a loop of `session.read` with
//!   `wait`, so one code path serves every node and nothing needs a
//!   subscription across hops. A wait returns within one round trip of a
//!   record landing; an empty page means nothing happened in that wait.
//!   A node answers at once, empty, when too many reads already wait on
//!   the connection or for this agent (protocol 6); the loop then pauses
//!   [`EARLY_PAUSE`] before the next, so that is a poll a second, not a
//!   busy loop.

mod args;
mod automation;
#[cfg(test)]
mod browser;
mod conversation;
mod forge;
mod forking;
mod lsp;
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
use vessel_proto::{FleetNode, KillSession, ListNodes, ListSessions, NodeId, SessionId};

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
        match name {
            name if name.starts_with("vessel_lsp_") => self.lsp(name, args).await,
            "list_nodes" => Ok(json!(self.client.call(ListNodes).await?)),
            "list_sessions" => {
                let node = self.target(args).await?;
                Ok(json!(self.on(node, ListSessions::default()).await?))
            }
            "list_projects" | "list_worktrees" | "create_worktree" => {
                self.workspace(name, args).await
            }
            "get_pull_request" | "open_pull_request" => self.forge(name, args).await,
            "search_sessions" => self.search(args).await,
            "fork_session" | "rewind_session" => self.forking(name, args).await,
            "create_session" => self.create_session(args).await,
            "send_input" => self.send_input(args).await,
            "answer_permission" => self.answer_permission(args).await,
            "read_events" => self.read_events(args).await,
            "wait_turn" => self.wait_turn(args).await,
            "read_conversation" | "change_queue" | "answer_question" | "resume_session"
            | "archive_session" => self.conversation(name, args).await,
            "create_terminal" | "list_terminals" | "read_terminal" | "write_terminal"
            | "close_terminal" | "list_ports" => self.terminals(name, args).await,
            "create_schedule" | "create_heartbeat" | "list_schedules" | "schedule_runs"
            | "pause_schedule" | "resume_schedule" | "delete_schedule" | "list_saved_prompts" => {
                self.automation(name, args).await
            }
            "kill_session" => {
                let node = self.target(args).await?;
                let session = session(args)?;
                let _ack = self.on(node, KillSession { session }).await?;
                Ok(json!({ "killed": session }))
            }
            other => Err(ToolError::Arguments(format!("no tool `{other}`"))),
        }
    }

    /// The node a `node` argument names: this one when it is left out or
    /// names this node; another by its id, or by its name in the fleet.
    async fn target(&self, args: &JsonObject) -> Result<Target, ToolError> {
        let Some(given) = text(args, "node")? else {
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
                 may send it input (queued or steered while it works), wait for its turns, \
                 answer it, stop it, archive it and start it again. To give one a folder and \
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
        Ok(ListToolsResult::with_all_items(tools()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let args = request.arguments.unwrap_or_default();
        match self.call(&request.name, &args).await {
            Ok(value) => Ok(CallToolResult::success(vec![McpContent::text(pretty(&value))]).into()),
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
    /// The call itself is wrong: a JSON-RPC error.
    #[error("{0}")]
    Arguments(String),
    /// Typed LSP arguments or results failed to encode/decode at the SDK edge.
    #[error("invalid LSP tool JSON")]
    LspJson(#[source] serde_json::Error),
    /// The node refused or failed; the agent reads why.
    #[error("{}", node_error(.0))]
    Node(#[from] CallError),
    /// The tool could not do what it was asked.
    #[error("{0}")]
    Failed(String),
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

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_unprintable| value.to_string())
}

/// Every tool with its input schema.
fn tools() -> Vec<Tool> {
    let session = json!({ "type": "string", "description": "A session id." });
    let node = node_schema();
    session_tools(&node, &session)
        .into_iter()
        .chain(conversation::tools(&node, &session))
        .chain(search::tools(&node, &session))
        .chain(forking::tools(&node, &session))
        .chain(workspace::tools(&node))
        .chain(forge::tools(&node, &session))
        .chain(terminals::tools(&node))
        .chain(automation::tools())
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
    properties: Value,
    required: &[&str],
) -> Tool {
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
