//! What the `vessel mcp` tests share: a node in this process with
//! `fake-acp` as Claude's adapter, the `vessel` server's environment as a
//! session's agent got it, and a client speaking MCP's JSON-RPC lines over
//! a pipe. Each test file takes it with `#[path]`.

use std::ffi::OsString;
use std::future::Future;
use std::time::Duration;

use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader, DuplexStream};
use vessel_client::{LocalCredential, NodeClient};
use vessel_harness::Lookup;
use vessel_harness::fakes::{Role, install};
use vessel_mcp::{AgentEnv, VesselTools};
use vessel_node::{Config, DataDir, Node, VesselMcp};
use vessel_proto::{
    AgentEvent, ClientInfo, ContentBlock, EventSeq, HarnessKind, ModelId, SessionEvent, SessionId,
    SessionInput, SessionMode, SessionSpec, SessionStatus,
};
use vessel_testkit::Fake;

pub(super) async fn within<F: Future>(test: F) -> F::Output {
    vessel_testkit::within(Duration::from_secs(60), Box::pin(test))
        .await
        .expect("test timed out")
}

/// A node with `fake-tui` as `claude` and `fake-acp` as its adapter,
/// attaching a `vessel` server (never run here) to every session.
pub(super) struct Fixture {
    pub(super) data: TempDir,
    _bin: TempDir,
    pub(super) node: Node,
}

impl Fixture {
    pub(super) async fn start() -> Self {
        Self::start_with(|_, _| {}).await
    }

    pub(super) async fn start_with(adjust: impl FnOnce(&mut Config, &std::path::Path)) -> Self {
        let data = tempfile::tempdir().unwrap();
        let bin = tempfile::tempdir().unwrap();
        let tui = Fake::Tui.path().unwrap();
        let acp = Fake::Acp.path().unwrap();
        drop(install(bin.path(), HarnessKind::ClaudeCode, Role::Terminal, &tui).unwrap());
        drop(install(bin.path(), HarnessKind::ClaudeCode, Role::Structured, &acp).unwrap());
        // A temporary home with no home variables, never the machine's.
        let user_home = data.path().join("user-home");
        std::fs::create_dir_all(&user_home).unwrap();
        let mut config = Config::new(
            DataDir::new(data.path()),
            vessel_harness::HarnessEnv::new(user_home),
        );
        config.bind = None;
        config.harness_lookup = Lookup::new([bin.path().to_path_buf()]);
        config.vessel_mcp = Some(VesselMcp::vessel(data.path().join("vessel")));
        adjust(&mut config, data.path());
        let node = Node::start(config).await.unwrap();
        Self {
            data,
            _bin: bin,
            node,
        }
    }
}

/// Serve `tools` over a pipe; the test's end of it, and the server's task.
pub(super) fn serve(
    tools: VesselTools,
) -> (
    McpClient,
    tokio::task::JoinHandle<Result<(), vessel_mcp::McpError>>,
) {
    let (server_end, client_end) = tokio::io::duplex(64 * 1024);
    let (read, write) = tokio::io::split(server_end);
    let server = tokio::spawn(vessel_mcp::serve(tools, read, write));
    (McpClient::new(client_end), server)
}

/// The test's end of the MCP pipe: one JSON-RPC message per line.
pub(super) struct McpClient {
    lines: tokio::io::Lines<BufReader<tokio::io::ReadHalf<DuplexStream>>>,
    write: tokio::io::WriteHalf<DuplexStream>,
    next: u64,
}

impl McpClient {
    pub(super) fn new(pipe: DuplexStream) -> Self {
        let (read, write) = tokio::io::split(pipe);
        Self {
            lines: BufReader::new(read).lines(),
            write,
            next: 1,
        }
    }

    pub(super) async fn notify(&mut self, method: &str) {
        let line = json!({ "jsonrpc": "2.0", "method": method }).to_string();
        self.write
            .write_all(format!("{line}\n").as_bytes())
            .await
            .unwrap();
    }

    /// Send a request; returns its response (`result` or `error`).
    pub(super) async fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next = self.next.saturating_add(1);
        let line =
            json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string();
        self.write
            .write_all(format!("{line}\n").as_bytes())
            .await
            .unwrap();
        loop {
            let line = self.lines.next_line().await.unwrap().expect("server left");
            let message: Value = serde_json::from_str(&line).unwrap();
            if message["id"] == json!(id) {
                return message;
            }
        }
    }

    /// `initialize`, then the tools' names.
    pub(super) async fn initialize(&mut self) -> Vec<String> {
        let initialized = self
            .request(
                "initialize",
                json!({
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": { "name": "test", "version": "0" }
                }),
            )
            .await;
        assert_eq!(initialized["result"]["serverInfo"]["name"], "vessel");
        self.notify("notifications/initialized").await;
        let listed = self.request("tools/list", json!({})).await;
        listed["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|tool| tool["name"].as_str().map(str::to_owned))
            .collect()
    }

    /// Call a tool; returns whether it failed and its text as JSON
    /// (or as a string, for a failure).
    pub(super) async fn call(&mut self, tool: &str, arguments: Value) -> (bool, Value) {
        let response = self
            .request(
                "tools/call",
                json!({ "name": tool, "arguments": arguments }),
            )
            .await;
        let result = &response["result"];
        assert!(result.is_object(), "{response}");
        let text = result["content"][0]["text"].as_str().unwrap();
        let failed = result["isError"] == json!(true);
        let value = if failed {
            Value::String(text.to_owned())
        } else {
            serde_json::from_str(text).unwrap()
        };
        (failed, value)
    }
}

pub(super) fn client() -> ClientInfo {
    ClientInfo {
        name: "vessel-mcp-test".to_owned(),
        version: "0".to_owned(),
    }
}

/// The node's owner, on the local socket.
pub(super) async fn owner(fixture: &Fixture) -> NodeClient {
    let token = DataDir::new(fixture.data.path())
        .read_token()
        .unwrap()
        .expose()
        .parse()
        .unwrap();
    NodeClient::connect_local(
        fixture.node.local_endpoint(),
        &LocalCredential::Token(token),
        client(),
    )
    .await
    .unwrap()
}

/// A structured session's `vessel` server environment, as its agent got
/// it: start one, wait until it runs, ask `fake-acp` for its servers.
pub(super) async fn agent_env(owner: &NodeClient) -> (SessionId, AgentEnv) {
    agent_env_with(owner, None).await
}

/// [`agent_env`], the session started with `profile`.
pub(super) async fn agent_env_with(
    owner: &NodeClient,
    profile: Option<vessel_proto::ProfileName>,
) -> (SessionId, AgentEnv) {
    let mut spec = SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Structured {
            model: Some(ModelId::new("fake-small")),
            resume: None,
            options: Vec::new(),
        },
    )
    .with_cwd(std::env::temp_dir());
    spec.profile = profile;
    let id = owner.create_session(spec).await.unwrap().id;
    let mut stream = owner.follow(id, EventSeq::ZERO).await.unwrap();
    let mut reply = String::new();
    let mut asked = false;
    while let Some(record) = stream.next().await {
        match record.unwrap().event {
            SessionEvent::Status {
                status: SessionStatus::Running,
            } if !asked => {
                asked = true;
                let prompt = SessionInput::Prompt {
                    content: vec![ContentBlock::Text {
                        text: "mcp".to_owned(),
                    }],
                };
                owner.send(id, prompt).await.unwrap();
            }
            SessionEvent::Agent {
                event:
                    AgentEvent::MessageChunk {
                        content: ContentBlock::Text { text },
                    },
            } => reply.push_str(&text),
            SessionEvent::Agent {
                event: AgentEvent::TurnEnded { .. },
            } => break,
            SessionEvent::Status { .. }
            | SessionEvent::Terminal(_)
            | SessionEvent::Agent { .. }
            | SessionEvent::ProfileChanged { .. }
            | SessionEvent::ModelCall(_)
            | SessionEvent::OptionsChanged { .. }
            | SessionEvent::Renamed { .. }
            | SessionEvent::QueueChanged { .. }
            | SessionEvent::CommandsChanged { .. }
            | SessionEvent::UsageChanged { .. }
            | SessionEvent::Parked { .. }
            | SessionEvent::Woken
            | SessionEvent::Forked { .. }
            | SessionEvent::Rewound { .. }
            | SessionEvent::Reloaded { .. } => {}
        }
    }
    let servers: Value = serde_json::from_str(reply.strip_prefix("mcp ").unwrap()).unwrap();
    let vessel = servers
        .as_array()
        .unwrap()
        .iter()
        .find(|server| server["name"] == "vessel")
        .unwrap()
        .clone();
    let env = AgentEnv::read(|name| {
        vessel["env"]
            .as_array()?
            .iter()
            .find(|variable| variable["name"] == name)
            .and_then(|variable| variable["value"].as_str())
            .map(OsString::from)
    })
    .unwrap();
    (id, env)
}

/// The tools as a structured session's agent has them, served; that
/// session's id, and the server's task.
pub(super) async fn as_agent(
    owner: &NodeClient,
) -> (
    McpClient,
    tokio::task::JoinHandle<Result<(), vessel_mcp::McpError>>,
    SessionId,
) {
    let (own_session, env) = agent_env(owner).await;
    let tools =
        VesselTools::new(vessel_mcp::connect(&env).await.unwrap()).with_own_session(own_session);
    let (mut mcp, server) = serve(tools);
    let _names = mcp.initialize().await;
    (mcp, server, own_session)
}
