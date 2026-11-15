//! Starting an ended structured session again (protocol 10's
//! `session.resume`, ADR 0026) in the session runtime, with `fake-acp` as
//! Claude's adapter: its log continues after its final status with the
//! agent's replay dropped, its title and choices kept, its queue back held;
//! and nothing is published until the node opens the gate.
//!
//! Tests wait on records, never on time; the timeout only turns a hang
//! into a failure.

use chrono::Utc;
use vessel_proto::{
    AgentEvent, ContentBlock, Creator, PromptQueue, QueueEntryId, QueueHold, QueuedPrompt,
    SessionEvent, SessionInfo, SessionStatus,
};
use vessel_session::{Created, Ended, Relaunch, Reopening, SessionHandle};

use support::{Fixture, Log, prompt, select, structured, within};

#[cfg(test)]
#[path = "support/runtime.rs"]
#[expect(
    dead_code,
    reason = "the shared runtime fixture, not every helper of it"
)]
mod support;

/// Helpers for this file. A `cfg(test)` module so the lints' test
/// allowances apply.
#[cfg(test)]
mod helpers {
    use super::*;

    /// A session that ran one turn (`plain`, which titles it) and was
    /// stopped, then let go by the runtime as the node does once the store
    /// has its log; its final info.
    pub(crate) async fn ended(fixture: &Fixture) -> SessionInfo {
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        log.until(running).await;
        handle.send(prompt("plain")).await.unwrap();
        log.turn_over().await;
        handle.kill();
        let info = handle.ended().await;
        drop(fixture.runtime.remove(info.id).unwrap());
        info
    }

    pub(crate) const fn turn_ended(event: &SessionEvent) -> bool {
        matches!(
            event,
            SessionEvent::Agent {
                event: AgentEvent::TurnEnded { .. }
            }
        )
    }

    pub(crate) const fn running(event: &SessionEvent) -> bool {
        matches!(
            event,
            SessionEvent::Status {
                status: SessionStatus::Running
            }
        )
    }

    /// A queue the node stopped before the session's end could hold.
    pub(crate) fn waiting() -> PromptQueue {
        PromptQueue {
            entries: vec![QueuedPrompt {
                provenance: None,
                id: QueueEntryId::random(),
                content: vec![ContentBlock::Text {
                    text: "later".to_owned(),
                }],
                queued_at: Utc::now(),
                revision: 0,
                by: Creator::Owner,

                client_message_id: None,
            }],
            held: None,
        }
    }

    /// Start `info`'s session again with `queue`; the handle, its records
    /// drained as the node's persistence would, and the gate.
    pub(crate) async fn again(
        fixture: &Fixture,
        info: SessionInfo,
        queue: PromptQueue,
    ) -> (SessionHandle, vessel_session::Gate) {
        let ended = Ended {
            info,
            choices: vec![select("mode", "plan")],
            queue,
            usage: None,
            attention: None,
            moved_through: std::collections::BTreeSet::new(),
        };
        let Reopening {
            created: Created { handle, durable },
            gate,
        } = fixture
            .runtime
            .start_again(ended, Relaunch::default())
            .await
            .unwrap();
        let mut durable = durable;
        drop(tokio::spawn(async move {
            while durable.recv().await.is_some() {}
        }));
        (handle, gate)
    }

    /// The message text of `log`'s records from `from` on.
    pub(crate) fn said(log: &Log, from: usize) -> String {
        log.records[from..]
            .iter()
            .filter_map(|record| {
                if let SessionEvent::Agent {
                    event:
                        AgentEvent::MessageChunk {
                            content: ContentBlock::Text { text },
                        },
                } = &record.event
                {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect()
    }
}

use helpers::{again, ended, running, said, turn_ended, waiting};

/// The same session continues: its log at the next record with
/// `starting`, its agent's replay of the conversation left out, `ready`
/// and `running` as for a new session; the agent loaded its conversation
/// with the session's choices; its title stays; its queue comes back held.
#[tokio::test]
async fn an_ended_session_continues_its_log() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let before = ended(&fixture).await;
        let next = before.next_seq;
        let conversation = before.harness_session.clone().unwrap();
        let (handle, gate) = again(&fixture, before.clone(), waiting()).await;
        let starting = handle.info();
        assert_eq!(starting.id, before.id);
        assert_eq!(starting.status, SessionStatus::Starting);
        assert_eq!(starting.next_seq, next);
        assert_eq!(starting.ended_at, None);
        assert_eq!(starting.title, before.title, "its title stays");
        assert_eq!(starting.created_at, before.created_at);
        let mut log = Log::from(&handle, next);
        assert!(log.records.is_empty(), "nothing before the gate opens");

        gate.open();
        log.until(running).await;
        let first = &log.records[0];
        assert_eq!(first.seq, next, "the log continues where it ended");
        assert!(matches!(
            first.event,
            SessionEvent::Status {
                status: SessionStatus::Starting
            }
        ));
        assert!(
            said(&log, 0).is_empty(),
            "the agent's replay is not published again: {:?}",
            log.records
        );
        assert!(log.records.iter().any(|record| matches!(
            &record.event,
            SessionEvent::Agent { event: AgentEvent::Ready { harness_session, .. } }
                if *harness_session == conversation
        )));
        let held = log.records.iter().find_map(|record| {
            if let SessionEvent::QueueChanged { queue } = &record.event {
                Some(queue.clone())
            } else {
                None
            }
        });
        assert_eq!(
            held.as_ref().and_then(|queue| queue.held),
            Some(QueueHold::Ended),
            "the queue comes back held"
        );
        assert_eq!(handle.conversation().unwrap().queue, held.unwrap());

        let asked = log.records.len();
        handle.send(prompt("loaded")).await.unwrap();
        log.since(asked, turn_ended).await;
        assert_eq!(said(&log, asked), format!("loaded {conversation}"));
        let asked = log.records.len();
        handle.send(prompt("options")).await.unwrap();
        log.since(asked, turn_ended).await;
        assert!(
            said(&log, asked).starts_with("options mode=plan "),
            "its choices applied after the load: {}",
            said(&log, asked)
        );
        assert_eq!(handle.info().title, before.title, "not renamed by a prompt");
        handle.kill();
        let after = handle.ended().await;
        assert!(after.next_seq > next);
    }))
    .await;
}

/// A session whose row the node could not reopen publishes nothing: its
/// gate dropped, it ends at once, its log untouched, and the runtime lets
/// it go.
#[tokio::test]
async fn a_session_not_reopened_publishes_nothing() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let before = ended(&fixture).await;
        let (handle, gate) = again(&fixture, before.clone(), PromptQueue::default()).await;
        drop(gate);
        let info = handle.ended().await;
        assert_eq!(info.next_seq, before.next_seq, "no record");
        let follow = handle.follow(before.next_seq).unwrap();
        assert!(follow.cached.is_empty());
        assert!(!handle.has_ended(), "no final status either");
        drop(fixture.runtime.remove(before.id).unwrap());
        assert!(fixture.runtime.get(before.id).is_none());
    }))
    .await;
}
