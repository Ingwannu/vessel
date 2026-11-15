//! Protocol 10 in the session runtime, with `fake-acp` as Claude's adapter:
//! the conversation a session keeps as it publishes (`session.conversation`).
//!
//! Tests wait on records, never on time; the timeout only turns a hang
//! into a failure.

use vessel_proto::{
    AgentAbilities, AgentEvent, AttachmentLimits, ContentBlock, Creator, Delivered, Delivery,
    ErrorCode, EventSeq, HarnessKind, PromptQueue, QueueChange, QueueHold, SessionEvent,
    SessionInput, SessionMode, SessionSpec, TerminalSize, TokenCount,
};
use vessel_session::{DeliveryError, SessionError};

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
                fork: true,
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

fn text(text: &str) -> Vec<ContentBlock> {
    vec![ContentBlock::Text {
        text: text.to_owned(),
    }]
}

fn turn_started(text: &str) -> impl Fn(&SessionEvent) -> bool {
    move |event| {
        matches!(event, SessionEvent::Agent { event: AgentEvent::TurnStarted { provenance: None, content ,
client_message_id: None,
} }
            if *content == self::text(text))
    }
}

fn turn_ended(event: &SessionEvent) -> bool {
    matches!(
        event,
        SessionEvent::Agent {
            event: AgentEvent::TurnEnded { .. }
        }
    )
}

/// Wait for `prompt`'s turn to start, then for its end.
async fn turn_of(log: &mut Log, prompt: &str) {
    log.next(turn_started(prompt)).await;
    let started = log.records.len().saturating_sub(1);
    log.since(started, turn_ended).await;
}

/// The queues the log holds, in order.
fn queues(log: &Log) -> Vec<PromptQueue> {
    log.records
        .iter()
        .filter_map(|record| {
            if let SessionEvent::QueueChanged { queue } = &record.event {
                Some(queue.clone())
            } else {
                None
            }
        })
        .collect()
}

/// A queued prompt waits for the running turn and starts after it, one
/// per turn; a steer joins the running turn; a turn not running is
/// started by a steer.
#[tokio::test]
async fn the_queue_moves_one_prompt_per_turn() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        let owner = || Creator::Owner;
        assert_eq!(
            handle
                .prompt(text("wait"), Delivery::Queue, owner())
                .await
                .unwrap(),
            Delivered::Started,
            "nothing runs, nothing waits"
        );
        let Delivered::Queued { entry } = handle
            .prompt(text("plain"), Delivery::Queue, owner())
            .await
            .unwrap()
        else {
            panic!("a turn runs: it waits");
        };
        let waiting = handle.conversation().unwrap().queue;
        assert_eq!(waiting.entries.len(), 1);
        assert_eq!(waiting.entries[0].id, entry);
        assert_eq!(
            handle
                .prompt(text("now"), Delivery::Steer, owner())
                .await
                .unwrap(),
            Delivered::Steered
        );
        assert_eq!(
            handle
                .prompt(text("finish"), Delivery::Steer, owner())
                .await
                .unwrap(),
            Delivered::Steered,
            "the fake ends its turn on this one"
        );
        turn_of(&mut log, "plain").await;
        // It left the queue (logged as its turn started, or as it ended).
        log.until(
            |event| matches!(event, SessionEvent::QueueChanged { queue } if queue.is_empty()),
        )
        .await;
        let steered = log
            .records
            .iter()
            .filter(|record| {
                matches!(
                    record.event,
                    SessionEvent::Agent {
                        event: AgentEvent::Steered { .. }
                    }
                )
            })
            .count();
        assert_eq!(steered, 2);
        assert_eq!(
            handle
                .prompt(text("plain"), Delivery::Steer, owner())
                .await
                .unwrap(),
            Delivered::Started,
            "no turn runs: a steer starts one"
        );
        assert!(matches!(
            handle.prompt(Vec::new(), Delivery::Start, owner()).await,
            Err(SessionError::Delivery(DeliveryError::Empty))
        ));
        handle.kill();
        let _end = handle.ended().await;
    }))
    .await;
}

/// A person's cancel holds what waits; `resume` starts it. An interrupt
/// cancels the running turn and starts its prompt, and holds nothing.
#[tokio::test]
async fn a_cancel_holds_the_queue_and_an_interrupt_does_not() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        let owner = || Creator::Owner;
        let _started = handle
            .prompt(text("wait"), Delivery::Start, owner())
            .await
            .unwrap();
        let _queued = handle
            .prompt(text("plain"), Delivery::Queue, owner())
            .await
            .unwrap();
        // A cancel naming a turn that is not the one running (the queue
        // started another meanwhile) holds the queue and nothing else.
        handle
            .send(SessionInput::CancelTurn {
                turn: Some(EventSeq::ZERO),
            })
            .await
            .unwrap();
        assert!(handle.turn_running().await.unwrap(), "it goes on");
        assert_eq!(
            handle.conversation().unwrap().queue.held,
            Some(QueueHold::Cancelled)
        );
        let moving = handle.change_queue(QueueChange::Resume).await.unwrap();
        assert_eq!(moving.delivered, None, "a turn runs: nothing starts");
        handle
            .send(SessionInput::CancelTurn { turn: None })
            .await
            .unwrap();
        log.next(turn_ended).await;
        let held = handle.conversation().unwrap().queue;
        assert_eq!(held.held, Some(QueueHold::Cancelled));
        assert_eq!(held.entries.len(), 1, "kept, not sent");

        let resumed = handle.change_queue(QueueChange::Resume).await.unwrap();
        assert_eq!(resumed.delivered, Some(Delivered::Started));
        assert!(resumed.queue.is_empty() && resumed.queue.held.is_none());
        turn_of(&mut log, "plain").await;

        let _started = handle
            .prompt(text("wait"), Delivery::Start, owner())
            .await
            .unwrap();
        let _queued = handle
            .prompt(text("plain"), Delivery::Queue, owner())
            .await
            .unwrap();
        assert_eq!(
            handle
                .prompt(text("options"), Delivery::Interrupt, owner())
                .await
                .unwrap(),
            Delivered::Started
        );
        turn_of(&mut log, "options").await;
        assert_eq!(
            handle.conversation().unwrap().queue.held,
            None,
            "an interrupt holds nothing"
        );
        // The interrupt's turn ends, and the queue's entry follows it.
        turn_of(&mut log, "plain").await;
        handle.kill();
        let _end = handle.ended().await;
    }))
    .await;
}

/// Entries are edited, moved and sent now by id; whatever waits when the
/// session ends is kept, held `ended`, before the final status.
#[tokio::test]
async fn entries_change_by_id_and_outlive_the_session_held() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        let owner = || Creator::Owner;
        let _started = handle
            .prompt(text("wait"), Delivery::Start, owner())
            .await
            .unwrap();
        let mut ids = Vec::new();
        for prompt in ["one", "two", "three"] {
            let Delivered::Queued { entry } = handle
                .prompt(text(prompt), Delivery::Queue, owner())
                .await
                .unwrap()
            else {
                panic!("queued");
            };
            ids.push(entry);
        }
        let moved = handle
            .change_queue(QueueChange::Move {
                entry: ids[2],
                before: Some(ids[0]),
            })
            .await
            .unwrap();
        let order: Vec<_> = moved.queue.entries.iter().map(|entry| entry.id).collect();
        assert_eq!(order, [ids[2], ids[0], ids[1]]);
        let edited = handle
            .change_queue(QueueChange::Edit {
                entry: ids[1],
                revision: 0,
                content: text("TWO"),
            })
            .await
            .unwrap();
        assert_eq!(edited.queue.entries[2].revision, 1);
        let stale = handle
            .change_queue(QueueChange::Edit {
                entry: ids[1],
                revision: 0,
                content: text("2"),
            })
            .await
            .unwrap_err();
        assert_eq!(stale.code(), ErrorCode::Conflict);
        let emptied = handle
            .change_queue(QueueChange::Edit {
                entry: ids[1],
                revision: 1,
                content: Vec::new(),
            })
            .await;
        assert!(
            matches!(emptied, Err(SessionError::Delivery(DeliveryError::Empty))),
            "an entry keeps content: {emptied:?}"
        );
        let sent = handle
            .change_queue(QueueChange::SendNow {
                entry: ids[0],
                delivery: Delivery::Steer,
            })
            .await
            .unwrap();
        assert_eq!(sent.delivered, Some(Delivered::Steered));
        assert_eq!(sent.queue.entries.len(), 2);
        let gone = handle
            .change_queue(QueueChange::Remove { entry: ids[0] })
            .await
            .unwrap_err();
        assert_eq!(gone.code(), ErrorCode::NotFound);

        handle.kill();
        let _end = handle.ended().await;
        log.until(|event| matches!(event, SessionEvent::Status { status } if !status.is_live()))
            .await;
        let last = queues(&log).pop().unwrap();
        assert_eq!(last.held, Some(QueueHold::Ended));
        assert_eq!(last.entries.len(), 2);
        let n = log.records.len();
        assert!(
            matches!(log.records[n - 2].event, SessionEvent::QueueChanged { .. }),
            "held before the final status"
        );
    }))
    .await;
}
