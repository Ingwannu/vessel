//! `agent.diagnostic` and `node.log` over the wire (ADR 0111, TRACKER 568,
//! 67, 87 and 569): an agent's facts with a new probe whose answer
//! `harness.options` takes next, agents that cannot run here, and the
//! node's log for an administrator only.

use vessel_proto::agents::{
    AgentDiagnostic, CatalogStatus, CommandSource, GetAgentDiagnostic, Installed, ProbeStep,
};
use vessel_proto::diagnostics::{NodeLog, ReadNodeLog};
use vessel_proto::{
    ErrorCode, HarnessKind, HarnessRef, OptionCategory, OptionKind, StructuredSupport,
};

use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::support::{Fixture, within};

fn diagnostic(kind: HarnessKind) -> GetAgentDiagnostic {
    GetAgentDiagnostic {
        agent: HarnessRef::Known(kind),
        profile: None,
    }
}

fn models(options: &vessel_proto::HarnessOptions) -> u32 {
    let Some(model) = options
        .options
        .iter()
        .find(|option| option.category == OptionCategory::Model)
    else {
        return 0;
    };
    let OptionKind::Select { values, .. } = &model.kind else {
        panic!("a model is a choice: {model:?}");
    };
    u32::try_from(values.len()).unwrap()
}

/// A ready agent: its command, where it is, each probe step, its models;
/// the probe is new even when `harness.options` had a fresh answer, and
/// `harness.options` takes the new one next.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_ready_agent_is_probed_again_and_its_answer_is_kept() {
    within(async {
        let fixture = Fixture::with_codex_terminal_only();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let before = client
            .harness_options(HarnessKind::ClaudeCode, None)
            .await
            .unwrap();

        let claude: AgentDiagnostic = client
            .call(diagnostic(HarnessKind::ClaudeCode))
            .await
            .unwrap();
        assert_eq!(claude.title, "Claude Code");
        assert_eq!(claude.command.source, CommandSource::BuiltIn);
        assert_eq!(claude.command.program, "claude");
        assert!(!claude.matches.is_empty(), "{claude:?}");
        assert!(
            matches!(
                claude.installed,
                Installed::Found {
                    structured: StructuredSupport::Available,
                    ..
                }
            ),
            "{claude:?}"
        );
        assert_eq!(
            (
                &claude.probe.spawn,
                &claude.probe.initialize,
                &claude.probe.session
            ),
            (&ProbeStep::Ok, &ProbeStep::Ok, &ProbeStep::Ok)
        );
        assert_eq!(
            claude.status,
            CatalogStatus::Ready {
                models: models(&before)
            }
        );
        let text = claude.text();
        assert!(text.starts_with("Claude Code\n"), "{text}");
        assert!(text.ends_with("  Status: Ready\n"), "{text}");

        // The diagnostic's probe replaced the kept answer.
        let after = client
            .harness_options(HarnessKind::ClaudeCode, None)
            .await
            .unwrap();
        assert!(after.probed_at > before.probed_at, "{before:?} {after:?}");
        let again = client
            .harness_options(HarnessKind::ClaudeCode, None)
            .await
            .unwrap();
        assert_eq!(again.probed_at, after.probed_at, "kept, not probed again");
        node.stop().await.unwrap();
    })
    .await;
}

/// An agent that runs no conversations here starts nothing; a missing one
/// fails to start; an unknown one is `not_found`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_that_cannot_run_here_say_why() {
    within(async {
        let fixture = Fixture::with_codex_terminal_only();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;

        let codex = client.call(diagnostic(HarnessKind::Codex)).await.unwrap();
        assert!(
            matches!(
                codex.installed,
                Installed::Found {
                    structured: StructuredSupport::Unavailable { .. },
                    ..
                }
            ),
            "{codex:?}"
        );
        assert_eq!(codex.probe.spawn, ProbeStep::NotReached);
        assert_eq!(codex.status, CatalogStatus::Unavailable);

        let gemini = client.call(diagnostic(HarnessKind::Gemini)).await.unwrap();
        assert!(
            matches!(gemini.installed, Installed::Missing { .. }),
            "{gemini:?}"
        );
        assert!(
            matches!(gemini.probe.spawn, ProbeStep::Failed { .. }),
            "{gemini:?}"
        );
        assert_eq!(gemini.probe.initialize, ProbeStep::NotReached);
        assert_eq!(gemini.status, CatalogStatus::Unavailable);
        assert!(gemini.matches.is_empty());

        let nobody = client
            .call(GetAgentDiagnostic {
                agent: HarnessRef::Added("nobody".parse().unwrap()),
                profile: None,
            })
            .await;
        assert_eq!(code(nobody), ErrorCode::NotFound);
        node.stop().await.unwrap();
    })
    .await;
}

/// The node's log for its owner; an agent gets neither call.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_log_is_for_administrators_and_agents_get_neither_call() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let owner = fixture.connect(&node).await;
        // This node runs inside the test process: its standard error is no
        // log file of its data directory.
        let log = owner.call(ReadNodeLog::default()).await.unwrap();
        assert!(matches!(log, NodeLog::NoFile { .. }), "{log:?}");
        assert_eq!(
            code(owner.call(ReadNodeLog { lines: Some(0) }).await),
            ErrorCode::InvalidRequest
        );

        let mut parent = structured(&owner, &fixture).await;
        let (_program, socket, auth) = credential(&owner, &mut parent).await;
        let agent = as_agent(&socket, &auth).await;
        assert_eq!(
            code(agent.call(diagnostic(HarnessKind::ClaudeCode)).await),
            ErrorCode::Forbidden
        );
        assert_eq!(
            code(agent.call(ReadNodeLog::default()).await),
            ErrorCode::Forbidden
        );
        node.stop().await.unwrap();
    })
    .await;
}
