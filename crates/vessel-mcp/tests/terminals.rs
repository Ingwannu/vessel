//! The terminal tools (protocol 11, ADR 0031) end to end, through a node in
//! this process: an agent whose session's profile allows it opens a shell
//! beside its own session, types into it, reads it and closes it; one whose
//! profile does not is refused. Unix only: the shell is `/bin/sh`.

#![cfg(unix)]

use serde_json::{Value, json};
use vessel_mcp::VesselTools;
use vessel_node::{DataDir, Shell};

use support::{Fixture, agent_env_with, owner, serve, within};

#[cfg(test)]
#[path = "support/mcp.rs"]
#[expect(
    dead_code,
    reason = "the shared MCP fixture, not every helper of it: this test serves tools of its own \
              (with the agent's session) instead of `as_agent`'s"
)]
mod support;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_allowed_agent_opens_types_into_reads_and_closes_a_terminal() {
    within(async {
        let fixture = Fixture::start_with(|config, data| {
            config.terminals.shell = Some(Shell::plain("/bin/sh"));
            std::fs::write(
                DataDir::new(data).profiles_path(),
                "[profiles.claude-term]\nharness = \"claude\"\nagent_terminals = true\n",
            )
            .unwrap();
        })
        .await;
        let owner = owner(&fixture).await;

        // A profile nobody marked: refused.
        let (_plain, env) = agent_env_with(&owner, None).await;
        let tools = VesselTools::new(vessel_mcp::connect(&env).await.unwrap())
            .with_own_session(env.auth.session);
        let (mut refused, refused_server) = serve(tools);
        let names = refused.initialize().await;
        for tool in [
            "create_terminal",
            "list_terminals",
            "read_terminal",
            "write_terminal",
            "close_terminal",
            "list_ports",
        ] {
            assert!(names.iter().any(|name| name == tool), "{tool}: {names:?}");
        }
        let (failed, answer) = refused.call("create_terminal", json!({})).await;
        assert!(failed, "{answer}");
        assert!(answer.as_str().unwrap().contains("forbidden"), "{answer}");
        drop(refused);
        refused_server.await.unwrap().unwrap();

        // A profile a person marked: its own terminal, beside its session.
        let (own, env) = agent_env_with(&owner, Some("claude-term".parse().unwrap())).await;
        let tools = VesselTools::new(vessel_mcp::connect(&env).await.unwrap())
            .with_own_session(env.auth.session);
        let (mut mcp, server) = serve(tools);
        let _names = mcp.initialize().await;
        let (failed, created) = mcp
            .call("create_terminal", json!({ "title": "check" }))
            .await;
        assert!(!failed, "{created}");
        let terminal = created["terminal"]["id"].clone();
        assert_eq!(
            created["terminal"]["place"]["session"],
            json!(own),
            "{created}"
        );

        let (failed, written) = mcp
            .call(
                "write_terminal",
                json!({ "terminal": terminal, "text": "echo mcp-$((6*7))", "keys": ["enter"] }),
            )
            .await;
        assert!(!failed, "{written}");
        let mut from = 0_u64;
        let mut text = String::new();
        while !text.contains("mcp-42") {
            let (failed, read): (bool, Value) = mcp
                .call(
                    "read_terminal",
                    json!({ "terminal": terminal, "from": from, "wait_secs": 5 }),
                )
                .await;
            assert!(!failed, "{read}");
            text.push_str(read["text"].as_str().unwrap());
            from = read["next"].as_u64().unwrap();
        }
        assert!(
            !text.contains('\u{1b}'),
            "escape sequences removed: {text:?}"
        );

        let (failed, listed) = mcp.call("list_terminals", json!({})).await;
        assert!(!failed, "{listed}");
        assert_eq!(listed["terminals"].as_array().unwrap().len(), 1, "{listed}");
        let (failed, ports) = mcp.call("list_ports", json!({})).await;
        assert!(!failed, "{ports}");
        let (failed, closed) = mcp
            .call("close_terminal", json!({ "terminal": terminal }))
            .await;
        assert!(!failed, "{closed}");
        let (failed, gone) = mcp
            .call("read_terminal", json!({ "terminal": terminal }))
            .await;
        assert!(failed, "{gone}");

        drop(mcp);
        server.await.unwrap().unwrap();
        fixture.node.stop().await.unwrap();
    })
    .await;
}
