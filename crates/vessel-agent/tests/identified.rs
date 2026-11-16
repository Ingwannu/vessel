//! Same text with distinct caller identities must produce distinct canonical echoes.
//! Only the deterministic testkit ACP/Codex processes are started here.

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use vessel_agent::{
        AgentOutput, AgentOutputs, AgentProcess, AgentSetup, Attachments, CommandSpec, Driver,
        Signal,
    };
    use vessel_proto::{AgentEvent, ContentBlock, timeline::ClientMessageId};
    use vessel_testkit::Fake;

    async fn until(outputs: &mut AgentOutputs, done: impl Fn(&AgentEvent) -> bool) -> AgentEvent {
        while let Some(output) = outputs.next().await {
            if let AgentOutput::InputEcho(echo) = output {
                let event = echo.into_event();
                if done(&event) {
                    return event;
                }
                continue;
            }
            if let AgentOutput::Event(event) = output {
                assert!(
                    !matches!(
                        event,
                        AgentEvent::TurnStarted { .. } | AgentEvent::Steered { .. }
                    ),
                    "accepted local inputs must use the trusted carrier"
                );
                if done(&event) {
                    return event;
                }
            }
        }
        panic!("fake provider closed before expected event");
    }

    fn setup() -> AgentSetup {
        AgentSetup {
            system_prompt: None,
            cwd: std::env::temp_dir(),
            model: None,
            resume: None,
            options: vec![],
            mcp_servers: vessel_proto::ResolvedMcpServers::new(),
            handshake_timeout: vessel_agent::DEFAULT_HANDSHAKE_TIMEOUT,
            questions: false,
            fork: None,
            private_dir: None,
        }
    }

    #[tokio::test]
    async fn echoes_copy_command_identity_instead_of_matching_prompt_text() {
        vessel_testkit::within(Duration::from_secs(30), async {
            for (driver, fake) in [
                (Driver::Acp, Fake::Acp),
                (Driver::CodexAppServer, Fake::CodexAppServer),
            ] {
                let mut command = CommandSpec::new(fake.path().unwrap());
                if driver == Driver::CodexAppServer {
                    command = command.arg("app-server");
                }
                let (mut agent, input, mut outputs) =
                    AgentProcess::spawn_with(driver, &command, setup()).unwrap();
                let _ready = until(&mut outputs, |event| {
                    matches!(event, AgentEvent::Ready { .. })
                })
                .await;
                let first = ClientMessageId::random();
                let second = ClientMessageId::random();
                assert_ne!(first, second);
                for message in [first, second] {
                    input
                        .prompt_identified(
                            vec![ContentBlock::Text {
                                text: "echo identical".into(),
                            }],
                            Attachments::new(),
                            Some(message.clone()),
                        )
                        .await
                        .unwrap();
                    let echo = until(&mut outputs, |event| {
                        matches!(event, AgentEvent::TurnStarted { .. })
                    })
                    .await;
                    let AgentEvent::TurnStarted {
                        client_message_id, ..
                    } = echo
                    else {
                        panic!("prompt echo");
                    };
                    assert_eq!(client_message_id, Some(message));
                    let _ended = until(&mut outputs, |event| {
                        matches!(event, AgentEvent::TurnEnded { .. })
                    })
                    .await;
                }
                agent.signal(Signal::Terminate).unwrap();
                let _code = agent.wait().await;
            }
        })
        .await
        .unwrap();
    }
}
