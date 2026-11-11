//! What an agent reports besides its timeline (protocol 10): what it can
//! do, the commands it offers, its context and a turn's tokens, against the
//! `fake-acp` testkit agent.

use std::time::Duration;

use vessel_agent::{AgentOutput, AgentProcess, CommandSpec, Signal};
use vessel_proto::{
    AgentAbilities, AgentCommand, AgentEvent, ContentBlock, ContextUsage, StopReason, TokenCount,
    TurnUsage,
};
use vessel_testkit::Fake;

use support::{is_ready, setup, until};

#[cfg(test)]
mod support {
    use vessel_agent::{AgentOutput, AgentOutputs, AgentSetup};
    use vessel_proto::AgentEvent;

    pub(super) fn setup() -> AgentSetup {
        AgentSetup {
            cwd: std::env::temp_dir(),
            model: None,
            resume: None,
            options: Vec::new(),
            mcp_servers: vessel_proto::ResolvedMcpServers::new(),
            handshake_timeout: vessel_agent::DEFAULT_HANDSHAKE_TIMEOUT,
        }
    }

    /// Every output until `done` says so of one.
    pub(super) async fn until(
        outputs: &mut AgentOutputs,
        done: impl Fn(&AgentOutput) -> bool,
    ) -> Vec<AgentOutput> {
        let mut seen = Vec::new();
        while let Some(output) = outputs.next().await {
            let stop = done(&output);
            seen.push(output);
            if stop {
                return seen;
            }
        }
        panic!("outputs ended first: {seen:?}");
    }

    pub(super) const fn is_ready(output: &AgentOutput) -> bool {
        matches!(output, AgentOutput::Event(AgentEvent::Ready { .. }))
    }
}

const SAFETY_NET: Duration = Duration::from_secs(30);

#[tokio::test]
async fn the_handshake_says_what_the_agent_can_do() {
    vessel_testkit::within(SAFETY_NET, async {
        let command = CommandSpec::new(Fake::Acp.path().unwrap());
        let (mut agent, _input, mut outputs) = AgentProcess::spawn(&command, setup()).unwrap();
        let seen = until(&mut outputs, is_ready).await;
        let Some(AgentOutput::Event(AgentEvent::Ready { abilities, .. })) = seen.last() else {
            panic!("{seen:?}");
        };
        assert_eq!(
            *abilities,
            Some(AgentAbilities {
                steer: true,
                resume: true,
                image: true,
                audio: false,
            })
        );
        agent.signal(Signal::Terminate).unwrap();
        let _code = agent.wait().await;

        // Without the steering extension in `_meta`, no steering.
        let command = command.env("FAKE_ACP_NO_STEER", "1");
        let (mut agent, _input, mut outputs) = AgentProcess::spawn(&command, setup()).unwrap();
        let seen = until(&mut outputs, is_ready).await;
        let Some(AgentOutput::Event(AgentEvent::Ready { abilities, .. })) = seen.last() else {
            panic!("{seen:?}");
        };
        assert_eq!(abilities.map(|abilities| abilities.steer), Some(false));
        agent.signal(Signal::Terminate).unwrap();
        let _code = agent.wait().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn commands_context_and_a_turns_tokens_are_reported() {
    vessel_testkit::within(SAFETY_NET, async {
        let command = CommandSpec::new(Fake::Acp.path().unwrap());
        let (mut agent, input, mut outputs) = AgentProcess::spawn(&command, setup()).unwrap();
        let mut seen = until(&mut outputs, is_ready).await;
        input
            .prompt(vec![ContentBlock::Text {
                text: "usage".to_owned(),
            }])
            .await
            .unwrap();
        seen.extend(
            until(&mut outputs, |output| {
                matches!(output, AgentOutput::Event(AgentEvent::TurnEnded { .. }))
            })
            .await,
        );
        let commands: Vec<&Vec<AgentCommand>> = seen
            .iter()
            .filter_map(|output| {
                if let AgentOutput::Commands(commands) = output {
                    Some(commands)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(commands.len(), 1, "reported once: {seen:?}");
        let names: Vec<&str> = commands[0].iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["compact", "review"]);
        assert_eq!(
            commands[0][0].hint.as_deref(),
            Some("<optional instructions>")
        );
        let usage: Vec<&ContextUsage> = seen
            .iter()
            .filter_map(|output| {
                if let AgentOutput::Usage(usage) = output {
                    Some(usage)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(usage.len(), 1);
        assert_eq!(usage[0].used, TokenCount::new(1200));
        assert_eq!(usage[0].size, TokenCount::new(200_000));
        assert_eq!(
            usage[0].cost.as_ref().map(|cost| cost.unit.as_str()),
            Some("USD")
        );
        let Some(AgentOutput::Event(AgentEvent::TurnEnded { reason, usage })) = seen.last() else {
            panic!("{seen:?}");
        };
        assert_eq!(*reason, StopReason::EndTurn);
        assert_eq!(
            *usage,
            Some(TurnUsage {
                total: TokenCount::new(30),
                input: TokenCount::new(20),
                output: TokenCount::new(10),
                thought: None,
                cached_read: None,
                cached_write: None,
            })
        );
        agent.signal(Signal::Terminate).unwrap();
        let _code = agent.wait().await;
    })
    .await
    .unwrap();
}
