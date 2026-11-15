//! Native policy bytes follow Paseo's version-gated presets, not ACP option ids.
#[cfg(test)]
mod tests {
    use std::time::Duration;
    use vessel_agent::{
        AgentOutput, AgentProcess, AgentSetup, CommandSpec, Disconnect, Driver, Signal,
    };
    use vessel_proto::{
        AgentEvent, ChoiceValue, ContentBlock, HarnessSessionId, OptionChoice, OptionId,
        OptionKind, OptionValueId, SessionOption,
    };

    fn setup(root: &std::path::Path, mode: Option<&str>, resume: bool) -> AgentSetup {
        AgentSetup {
            system_prompt: None,
            cwd: root.to_owned(),
            model: None,
            resume: resume.then(|| HarnessSessionId::new("synthetic-policy-thread")),
            fork: None,
            options: mode
                .map(|value| OptionChoice {
                    id: OptionId::new("mode"),
                    value: ChoiceValue::Select {
                        value: OptionValueId::new(value),
                    },
                })
                .into_iter()
                .collect(),
            mcp_servers: Default::default(),
            handshake_timeout: Duration::from_secs(5),
            questions: false,
            private_dir: None,
        }
    }

    fn command(root: &std::path::Path, version: &str) -> CommandSpec {
        CommandSpec::new(vessel_testkit::Fake::CodexAppServer.path().unwrap())
            .arg("app-server")
            .env_clear()
            .env(if cfg!(windows) { "USERPROFILE" } else { "HOME" }, root)
            .env("FAKE_CODEX_VERSION", version)
            .env("FAKE_CODEX_LAUNCH_AUDIT", root.join("launch.json"))
            .env("FAKE_CODEX_TURN_AUDIT", root.join("turn.json"))
    }

    async fn observe(
        version: &str,
        mode: Option<&str>,
        resume: bool,
    ) -> (serde_json::Value, serde_json::Value, Vec<SessionOption>) {
        let root = tempfile::tempdir().unwrap();
        let (mut agent, input, mut outputs) = AgentProcess::spawn_with(
            Driver::CodexAppServer,
            &command(root.path(), version),
            setup(root.path(), mode, resume),
        )
        .unwrap();
        let mut options = Vec::new();
        loop {
            match outputs.next().await.unwrap() {
                AgentOutput::Options(next) => options = next,
                AgentOutput::Event(AgentEvent::Ready { .. }) => break,
                AgentOutput::Disconnected(error) => panic!("fake handshake refused: {error:?}"),
                AgentOutput::Event(_)
                | AgentOutput::Commands(_)
                | AgentOutput::Usage(_)
                | AgentOutput::Introduced(_) => {}
            }
        }
        input
            .prompt(vec![ContentBlock::Text {
                text: "policy".into(),
            }])
            .await
            .unwrap();
        loop {
            match outputs.next().await.unwrap() {
                AgentOutput::Event(AgentEvent::TurnEnded { .. }) => break,
                AgentOutput::Disconnected(error) => panic!("fake turn disconnected: {error:?}"),
                AgentOutput::Event(_)
                | AgentOutput::Options(_)
                | AgentOutput::Commands(_)
                | AgentOutput::Usage(_)
                | AgentOutput::Introduced(_) => {}
            }
        }
        let launch =
            serde_json::from_slice(&std::fs::read(root.path().join("launch.json")).unwrap())
                .unwrap();
        let turn =
            serde_json::from_slice(&std::fs::read(root.path().join("turn.json")).unwrap()).unwrap();
        agent.signal(Signal::Terminate).unwrap();
        let _exit = agent.wait().await;
        (launch, turn, options)
    }

    fn selections(options: &[SessionOption]) -> (&str, Vec<&str>) {
        let option = options
            .iter()
            .find(|option| option.id.as_str() == "mode")
            .unwrap();
        let OptionKind::Select { current, values } = &option.kind else {
            panic!("native mode is not a selector");
        };
        (
            current.as_str(),
            values.iter().map(|value| value.id.as_str()).collect(),
        )
    }

    #[tokio::test]
    async fn native_defaults_and_offered_ids_follow_the_observed_version_floor() {
        vessel_testkit::within(Duration::from_secs(30), async {
            for (version, current, offered, reviewer) in [
                ("0.114.9", "auto", vec!["auto", "full-access"], "user"),
                ("unknown", "auto", vec!["auto", "full-access"], "user"),
                (
                    "0.115.0",
                    "auto-review",
                    vec!["auto", "auto-review", "full-access"],
                    "auto_review",
                ),
            ] {
                let (launch, turn, options) = observe(version, None, false).await;
                assert_eq!(selections(&options), (current, offered));
                assert_eq!(launch["approvalPolicy"], "on-request");
                assert_eq!(launch["approvalsReviewer"], reviewer);
                assert_eq!(launch["sandbox"], "workspace-write");
                assert_eq!(turn["approvalPolicy"], "on-request");
                assert_eq!(turn["approvalsReviewer"], reviewer);
                assert_eq!(
                    turn["sandboxPolicy"],
                    serde_json::json!({"type":"workspaceWrite","networkAccess":false})
                );
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn legacy_native_aliases_preserve_exact_policy_on_start_and_resume_without_being_offered()
    {
        vessel_testkit::within(Duration::from_secs(30), async {
            for (legacy, current, approval, reviewer, sandbox, turn_sandbox) in [
                (
                    "workspace-write",
                    "auto",
                    "on-request",
                    "user",
                    "workspace-write",
                    serde_json::json!({"type":"workspaceWrite","networkAccess":false}),
                ),
                (
                    "agent",
                    "auto-review",
                    "on-request",
                    "auto_review",
                    "workspace-write",
                    serde_json::json!({"type":"workspaceWrite","networkAccess":false}),
                ),
                (
                    "agent-full-access",
                    "full-access",
                    "never",
                    "user",
                    "danger-full-access",
                    serde_json::json!({"type":"dangerFullAccess"}),
                ),
                (
                    "read-only",
                    "read-only",
                    "on-request",
                    "user",
                    "read-only",
                    serde_json::json!({"type":"readOnly","networkAccess":false}),
                ),
            ] {
                for resume in [false, true] {
                    let (launch, turn, options) = observe("0.166.0", Some(legacy), resume).await;
                    assert_eq!(
                        selections(&options),
                        (current, vec!["auto", "auto-review", "full-access"])
                    );
                    assert_eq!(launch.get("threadId").is_some(), resume);
                    assert_eq!(launch["approvalPolicy"], approval);
                    assert_eq!(launch["approvalsReviewer"], reviewer);
                    assert_eq!(launch["sandbox"], sandbox);
                    assert_eq!(turn["approvalPolicy"], approval);
                    assert_eq!(turn["approvalsReviewer"], reviewer);
                    assert_eq!(turn["sandboxPolicy"], turn_sandbox);
                }
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn unavailable_auto_review_refuses_before_a_native_thread_can_be_created() {
        vessel_testkit::within(Duration::from_secs(30), async {
            let root = tempfile::tempdir().unwrap();
            let (mut agent, _input, mut outputs) = AgentProcess::spawn_with(
                Driver::CodexAppServer,
                &command(root.path(), "0.114.9"),
                setup(root.path(), Some("agent"), false),
            )
            .unwrap();
            loop {
                if let AgentOutput::Disconnected(error) = outputs.next().await.unwrap() {
                    assert!(matches!(error, Disconnect::Handshake { .. }));
                    break;
                }
            }
            assert!(!root.path().join("launch.json").exists());
            assert!(!root.path().join("turn.json").exists());
            let _exit = agent.wait().await;
        })
        .await
        .unwrap();
    }
}
