//! `vessel mcp` end to end: a node in this process (local socket only),
//! a structured session whose agent got the `vessel` server (`fake-acp`
//! reports it, values included), and the tools served over a pipe to a test
//! that speaks MCP's JSON-RPC lines, as a harness would.
//!
//! The environment is read with this crate's variable names from what the
//! node actually set, so the two cannot drift apart unnoticed.

use std::io::Write as _;
use std::time::Duration;

use serde_json::json;
use vessel_mcp::VesselTools;
use vessel_proto::{EventSeq, SessionEvent};

use support::{Fixture, McpClient, agent_env, owner, within};

#[cfg(test)]
#[path = "support/mcp.rs"]
#[expect(dead_code, reason = "the shared fixture, not every helper of it")]
mod support;

/// The tools before LSP's, in the order they are listed.
const TOOLS: &[&str] = &[
    "list_nodes",
    "list_sessions",
    "create_session",
    "send_input",
    "answer_permission",
    "read_events",
    "wait_turn",
    "kill_session",
    "get_session",
    "cancel_turn",
    "rename_session",
    "set_session_option",
    "read_conversation",
    "change_queue",
    "answer_question",
    "resume_session",
    "archive_session",
    "search_sessions",
    "fork_session",
    "rewind_session",
    "list_projects",
    "list_worktrees",
    "create_worktree",
    "get_pull_request",
    "open_pull_request",
    "create_terminal",
    "list_terminals",
    "read_terminal",
    "write_terminal",
    "close_terminal",
    "list_ports",
    "create_schedule",
    "create_heartbeat",
    "list_schedules",
    "schedule_runs",
    "pause_schedule",
    "resume_schedule",
    "delete_schedule",
    "list_saved_prompts",
    "get_schedule",
    "update_schedule",
    "list_harnesses",
    "list_profiles",
    "harness_options",
    "list_pending_permissions",
];

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_starts_a_session_reads_its_reply_and_stops_it() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = owner(&fixture).await;
        let (own_session, env) = agent_env(&owner).await;
        assert_eq!(env.auth.session, own_session);

        let tools = VesselTools::new(vessel_mcp::connect(&env).await.unwrap());
        let (server_end, client_end) = tokio::io::duplex(64 * 1024);
        let (read, write) = tokio::io::split(server_end);
        let server = tokio::spawn(vessel_mcp::serve(tools, read, write));
        let mut mcp = McpClient::new(client_end);

        // The session tools (protocol 4, 10), then a session's record, cancel,
        // title and option; the search (protocol 12); the project and worktree
        // tools (protocol 10); the pull request; terminal and schedule tools
        // (protocol 11), including one schedule's record and its replacement;
        // harnesses, profiles and a harness's options; then LSP's.
        let names = mcp.initialize().await;
        assert_eq!(&names[..TOOLS.len()], TOOLS);
        assert_eq!(names.len(), 55);
        assert!(names.iter().any(|name| name == "vessel_lsp_diagnostics"));

        // Not in a fleet: a fleet of one, this node.
        let (failed, nodes) = mcp.call("list_nodes", json!({})).await;
        assert!(!failed);
        assert_eq!(nodes.as_array().unwrap().len(), 1, "{nodes}");
        assert_eq!(nodes[0]["node"], json!(fixture.node.info().id));
        assert_eq!(nodes[0]["reachable"], json!(true));

        // A conversation of its own: started, prompted, answered.
        let (failed, created) = mcp
            .call(
                "create_session",
                json!({ "harness": "claude", "cwd": std::env::temp_dir(), "prompt": "plain" }),
            )
            .await;
        assert!(!failed, "{created}");
        let session = created["session"]["id"].clone();
        assert_eq!(created["session"]["created_by"]["kind"], "agent");
        assert_eq!(
            created["session"]["created_by"]["session"],
            json!(own_session)
        );
        let (failed, turn) = mcp
            .call(
                "wait_turn",
                json!({ "session": session, "from": created["from"], "timeout_secs": 30 }),
            )
            .await;
        assert!(!failed, "{turn}");
        assert_eq!(turn["outcome"], "ended");
        assert_eq!(turn["reply"], "echo: plain");

        let (failed, sessions) = mcp.call("list_sessions", json!({})).await;
        assert!(!failed);
        assert!(sessions.as_array().unwrap().len() >= 2);

        drive_session_and_check_ownership(&mut mcp, &session, own_session).await;
        provider_discovery(&mut mcp).await;

        // Not its own session, nor the user's: the node says why.
        let (failed, refusal) = mcp
            .call("kill_session", json!({ "session": own_session }))
            .await;
        assert!(failed);
        assert!(
            refusal.as_str().unwrap().starts_with("\"forbidden\""),
            "{refusal}"
        );
        let (failed, _killed) = mcp
            .call("kill_session", json!({ "session": session }))
            .await;
        assert!(!failed);

        // A node not in the fleet, a wait that does not say where from, and
        // an unknown tool: errors of the call.
        for (tool, arguments) in [
            ("list_sessions", json!({ "node": "elsewhere" })),
            ("wait_turn", json!({ "session": session })),
            ("nope", json!({})),
        ] {
            let response = mcp
                .request(
                    "tools/call",
                    json!({ "name": tool, "arguments": arguments }),
                )
                .await;
            assert!(response["error"].is_object(), "{response}");
        }

        drop(mcp);
        tokio::time::timeout(Duration::from_secs(10), server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        owner.kill(own_session).await.unwrap();
        fixture.node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_ended_sessions_credential_is_refused() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = owner(&fixture).await;
        let (session, env) = agent_env(&owner).await;
        owner.kill(session).await.unwrap();
        let mut stream = owner.follow(session, EventSeq::ZERO).await.unwrap();
        while let Some(record) = stream.next().await {
            if matches!(record.unwrap().event, SessionEvent::Status { status } if !status.is_live())
            {
                break;
            }
        }
        assert!(vessel_mcp::connect(&env).await.is_err());
        fixture.node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_uses_every_lsp_tool_through_mcp() {
    within(async {
        // A stand-in Rust toolchain outside the session's root.
        let tools = tempfile::tempdir().unwrap();
        let fixture = Fixture::start_with(|config, data| {
            std::fs::write(data.join("fixture.rs"), "pub fn caller() { target(); }\npub fn target() {}\n").unwrap();
            std::fs::write(data.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").unwrap();
            config.lsp.registry.rust_toolchain = Some(vessel_testkit::rust_toolchain(tools.path()).unwrap());
            let _previous = config.lsp.registry.overrides.insert(vessel_proto::lsp::Language::Rust, vessel_lsp::registry::Override { program: vessel_testkit::Fake::Lsp.path().unwrap(), args: vec!["normal".into()] });
        }).await;
        let owner = owner(&fixture).await;
        let (_, env) = agent_env(&owner).await;
        let tools = VesselTools::new(vessel_mcp::connect(&env).await.unwrap());
        let (server_end, client_end) = tokio::io::duplex(64 * 1024);
        let (read, write) = tokio::io::split(server_end);
        let server = tokio::spawn(vessel_mcp::serve(tools, read, write));
        let mut mcp = McpClient::new(client_end);
        let names = mcp.initialize().await;
        let workspace = json!({"root":fixture.data.path(),"language":"rust"});
        for name in names.iter().filter(|name| name.starts_with("vessel_lsp_")) {
            let args = json!({"workspace":workspace,"file":"fixture.rs","position":{"line":2,"column":8},"query":"target"});
            let (failed, reply) = mcp.call(name, args).await;
            assert!(!failed, "{name}: {reply}");
            assert!(reply["context"]["files"].is_array(), "{name}: {reply}");
            assert!(reply["context"]["observed_at"].is_string(), "{name}: {reply}");
        }
        drop(mcp);
        server.await.unwrap().unwrap();
        drop(owner);
        fixture.node.stop().await.unwrap();
    }).await;
}

/// Optional real-server probe; never runs in the ordinary account-free gate.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires rust-analyzer on PATH; run docs/smoke/m6-lsp.py"]
async fn real_rust_analyzer_context_through_agent_mcp() {
    within(async {
        let executable = which::which("rust-analyzer").expect("rust-analyzer must be on PATH");
        let sysroot = std::process::Command::new("rustc").args(["--print", "sysroot"]).output().unwrap();
        assert!(sysroot.status.success());
        let mut toolchain = std::path::PathBuf::from(String::from_utf8(sysroot.stdout).unwrap().trim());
        if let Ok(server) = executable.canonicalize()
            && let Some(candidate) = server.parent().and_then(std::path::Path::parent)
            && candidate.join("lib/rustlib/src/rust/library").is_dir() {
            toolchain = candidate.to_owned();
        }
        let cargo_home = std::env::var_os("CARGO_HOME").map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".cargo")));
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().canonicalize().unwrap();
        let fixture = Fixture::start_with(|config, _| {
            config.lsp.registry.rust_toolchain = Some(toolchain);
            config.lsp.registry.cargo_home = cargo_home;
            config.lsp.registry.system_root = std::env::var_os("SystemRoot");
            config.lsp_roots.push(vessel_node::LspRoot { path: root.clone(), full_analysis: false });
            let _previous = config.lsp.registry.overrides.insert(vessel_proto::lsp::Language::Rust, vessel_lsp::registry::Override { program: executable, args: Vec::new() });
        }).await;
        let owner = owner(&fixture).await;
        let (_, env) = agent_env(&owner).await;
        let tools = VesselTools::new(vessel_mcp::connect(&env).await.unwrap());
        let (server_end, client_end) = tokio::io::duplex(128 * 1024);
        let (read, write) = tokio::io::split(server_end);
        let server = tokio::spawn(vessel_mcp::serve(tools, read, write));
        let mut mcp = McpClient::new(client_end);
        let _tools = mcp.initialize().await;
        let file = "crates/vessel-lsp/src/position.rs";
        let source = std::fs::read_to_string(root.join(file)).unwrap();
        let line = source.lines().position(|line| line.starts_with("pub fn to_utf16(")).unwrap().saturating_add(1);
        let workspace = json!({"root":root,"language":"rust"});
        for name in ["document_symbols", "workspace_symbols", "definition", "references", "hover", "implementations", "type_definition", "incoming_calls", "outgoing_calls", "diagnostics"] {
            let name = format!("vessel_lsp_{name}");
            let args = json!({"workspace":workspace,"file":file,"position":{"line":line,"column":8},"query":"to_utf16"});
            let (failed, reply) = mcp.call(&name, args).await;
            // Unsupported is an honest feature outcome; every supported
            // response still has coverage and must survive MCP serialization.
            if failed { assert!(reply.as_str().is_some_and(|text| text.contains("unsupported")), "{name}: {reply}"); }
            else { assert!(reply["context"].is_object(), "{name}: {reply}"); }
            let record = json!({"tool":name,"failed":failed,"context":reply.get("context"),"items":reply.get("items").and_then(|items| items.as_array()).map(Vec::len)});
            writeln!(std::io::stdout().lock(), "{record}").unwrap();
            if name == "vessel_lsp_hover" { assert!(reply["text"].as_str().is_some_and(|text| !text.is_empty()), "semantic hover was empty: {reply}"); }
            if name == "vessel_lsp_workspace_symbols" { assert!(!reply["items"].as_array().unwrap().is_empty(), "workspace was not indexed: {reply}"); }
            if name == "vessel_lsp_document_symbols" { assert!(!failed); assert!(!reply["items"].as_array().unwrap().is_empty()); }
        }
        drop(mcp);
        server.await.unwrap().unwrap();
        drop(owner);
        fixture.node.stop().await.unwrap();
    }).await;
}

#[cfg(test)]
async fn provider_discovery(mcp: &mut McpClient) {
    let (failed, harnesses) = mcp.call("list_harnesses", json!({})).await;
    assert!(!failed, "{harnesses}");
    let harnesses = harnesses.as_array().expect("harnesses");
    assert!(!harnesses.is_empty(), "{harnesses:?}");
    for one in harnesses {
        assert!(
            one.get("kind").is_some() && one.get("path").is_some(),
            "{one}"
        );
    }
    let (failed, profiles) = mcp.call("list_profiles", json!({})).await;
    assert!(!failed, "{profiles}");
    for profile in profiles.as_array().expect("profiles") {
        for key in profile.as_object().expect("profile").keys() {
            assert!(
                matches!(
                    key.as_str(),
                    "name" | "label" | "harness" | "sets" | "model"
                ),
                "{key} in {profile}"
            );
        }
    }
    let (failed, options) = mcp
        .call("harness_options", json!({ "harness": "claude" }))
        .await;
    assert!(!failed, "{options}");
    assert!(options["options"].is_array(), "{options}");
    assert!(options.get("probed_at").is_some(), "{options}");
}

#[cfg(test)]
async fn drive_session_and_check_ownership(
    mcp: &mut McpClient,
    session: &serde_json::Value,
    own_session: vessel_proto::SessionId,
) {
    let (failed, got) = mcp.call("get_session", json!({ "session": session })).await;
    assert!(!failed, "{got}");
    assert_eq!(&got["id"], session);
    assert!(got["permissions"].is_array(), "{got}");
    let (failed, renamed) = mcp
        .call(
            "rename_session",
            json!({ "session": session, "title": "from the agent" }),
        )
        .await;
    assert!(!failed, "{renamed}");
    assert_eq!(renamed["title"], "from the agent");
    for (tool, arguments) in [
        (
            "rename_session",
            json!({ "session": own_session, "title": "no" }),
        ),
        ("cancel_turn", json!({ "session": own_session })),
        (
            "set_session_option",
            json!({ "session": own_session, "option": "mode", "value": "plan" }),
        ),
    ] {
        let (failed, refusal) = mcp.call(tool, arguments).await;
        assert!(failed, "{tool}");
        assert!(
            refusal.as_str().unwrap().starts_with("\"forbidden\""),
            "{tool}: {refusal}"
        );
    }
}
