//! Exact native thread launch payloads, using only testkit and synthetic values.

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, time::Duration};
    use vessel_agent::{AgentOutput, AgentProcess, AgentSetup, CommandSpec, Driver, Signal};
    use vessel_proto::{
        AgentEvent, HarnessSessionId, McpTransport, ResolvedMcpServers, ResolvedValue,
    };

    const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
    const SAFETY_NET: Duration = Duration::from_secs(30);

    fn servers() -> ResolvedMcpServers {
        BTreeMap::from([
            (
                "local".parse().unwrap(),
                McpTransport::Stdio {
                    command: "synthetic-helper".into(),
                    args: vec!["literal argument".into()],
                    env: BTreeMap::from([(
                        "SYNTHETIC_TOKEN".parse().unwrap(),
                        ResolvedValue::new("fake-stdio-value".into()),
                    )]),
                },
            ),
            (
                "remote".parse().unwrap(),
                McpTransport::Http {
                    url: "https://synthetic.invalid/mcp".parse().unwrap(),
                    headers: BTreeMap::from([(
                        "authorization".parse().unwrap(),
                        ResolvedValue::new("Bearer fake-http-value".into()),
                    )]),
                },
            ),
        ])
    }

    async fn launch(resume: bool, configured: bool) -> serde_json::Value {
        let root = tempfile::tempdir().unwrap();
        let audit = root.path().join("thread.json");
        let command = CommandSpec::new(vessel_testkit::Fake::CodexAppServer.path().unwrap())
            .arg("app-server")
            .env_clear()
            .env(
                if cfg!(windows) { "USERPROFILE" } else { "HOME" },
                root.path(),
            )
            .env("FAKE_CODEX_LAUNCH_AUDIT", &audit);
        let setup = AgentSetup {
            system_prompt: None,
            cwd: root.path().to_owned(),
            model: None,
            resume: resume.then(|| HarnessSessionId::new("synthetic-resumed-thread")),
            fork: None,
            options: vec![],
            mcp_servers: if configured {
                servers()
            } else {
                ResolvedMcpServers::new()
            },
            handshake_timeout: HANDSHAKE_TIMEOUT,
            questions: false,
            private_dir: None,
        };
        let (mut agent, _input, mut outputs) =
            AgentProcess::spawn_with(Driver::CodexAppServer, &command, setup).unwrap();
        loop {
            match outputs.next().await.unwrap() {
                AgentOutput::Event(AgentEvent::Ready { .. }) => break,
                AgentOutput::Disconnected(why) => panic!("fake handshake failed: {why:?}"),
                AgentOutput::Event(_)
                | AgentOutput::Options(_)
                | AgentOutput::Commands(_)
                | AgentOutput::Usage(_)
                | AgentOutput::Introduced(_) => {}
            }
        }
        let payload = serde_json::from_slice(&std::fs::read(audit).unwrap()).unwrap();
        agent.signal(Signal::Terminate).unwrap();
        let _exit = agent.wait().await;
        payload
    }

    #[tokio::test]
    async fn native_mcp_config_is_exact_on_start_and_resume_and_absent_when_unconfigured() {
        vessel_testkit::within(SAFETY_NET, async {
        let expected = serde_json::json!({"mcp_servers": {
            "local": {"command":"synthetic-helper","args":["literal argument"],"env":{"SYNTHETIC_TOKEN":"fake-stdio-value"}},
            "remote": {"url":"https://synthetic.invalid/mcp","http_headers":{"authorization":"Bearer fake-http-value"}}
        }});
        for resume in [false, true] {
            let configured = launch(resume, true).await;
            assert_eq!(configured["config"], expected);
            assert_eq!(configured.get("threadId").is_some(), resume);
            assert!(launch(resume, false).await.get("config").is_none());
        }
    }).await.unwrap();
    }
}
