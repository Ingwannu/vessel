//! Protocol 10 in the session runtime, with `fake-acp` as Claude's adapter:
//! the conversation a session keeps as it publishes (`session.conversation`).
//!
//! Tests wait on records, never on time; the timeout only turns a hang
//! into a failure.

use vessel_proto::{
    AgentAbilities, AgentEvent, AttachmentLimits, ErrorCode, HarnessKind, SessionEvent,
    SessionMode, SessionSpec, TerminalSize, TokenCount,
};
use vessel_session::SessionError;

use support::{Fixture, Log, prompt, structured, within};

#[cfg(test)]
#[path = "support/runtime.rs"]
#[expect(
    dead_code,
    reason = "the shared runtime fixture, not every helper of it"
)]
mod support;

/// What the agent said while starting (its abilities, its commands) and
/// after a turn (its context) is there in one call, without the log.
#[tokio::test]
async fn the_conversation_holds_what_the_agent_reported() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        // The agent sends its commands when it likes, here after `Ready`
        // or before it, as Claude's adapter does.
        log.until(|event| matches!(event, SessionEvent::CommandsChanged { .. }))
            .await;
        log.until(|event| {
            matches!(
                event,
                SessionEvent::Agent {
                    event: AgentEvent::Ready { .. }
                }
            )
        })
        .await;
        let started = handle.conversation().unwrap();
        assert_eq!(
            started.abilities,
            Some(AgentAbilities {
                steer: true,
                resume: true,
                image: true,
                audio: false,
            })
        );
        let names: Vec<&str> = started.commands.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["compact", "review"]);
        assert_eq!(started.usage, None, "no report yet");
        assert!(started.queue.entries.is_empty());
        assert_eq!(started.limits, AttachmentLimits::DEFAULT);

        handle.send(prompt("usage")).await.unwrap();
        log.until(|event| {
            matches!(
                event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnEnded { usage: Some(_), .. }
                }
            )
        })
        .await;
        let usage = handle.conversation().unwrap().usage.unwrap();
        assert_eq!(usage.used, TokenCount::new(1200));
        assert_eq!(usage.size, TokenCount::new(200_000));

        handle.kill();
        let _end = handle.ended().await;
        let ended = handle.conversation().unwrap();
        assert!(ended.commands.is_empty(), "an ended session offers none");
        assert_eq!(ended.usage, Some(usage), "its last figure stays");
    }))
    .await;
}

/// A terminal session's harness runs its own interface: no conversation.
#[tokio::test]
async fn a_terminal_session_has_no_conversation() {
    within(async {
        let fixture = Fixture::new();
        let spec = SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Terminal {
                args: Vec::new(),
                size: TerminalSize::default(),
            },
        )
        .with_cwd(std::env::temp_dir());
        let handle = fixture.start(spec).await;
        let error = handle.conversation().unwrap_err();
        assert!(matches!(error, SessionError::Terminal(_)), "{error:?}");
        assert_eq!(error.code(), ErrorCode::Unsupported);
        handle.kill();
        let _end = handle.ended().await;
    })
    .await;
}
