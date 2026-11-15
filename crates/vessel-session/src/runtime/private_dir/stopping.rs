//! Hold the real journal ACK while the fake agent and actor remain live.
use super::*;
use crate::{JournalEntry, SessionStop};
use std::task::Poll;
use tokio::sync::mpsc;
use vessel_proto::restart::RestartCheckpoint;
use vessel_proto::{Creator, Delivered, Delivery, SessionTitle};

/// Safety net for the independent actor barrier, not a product deadline.
const RENAME_BARRIER: Duration = Duration::from_secs(20);

fn text() -> Vec<ContentBlock> {
    vec![ContentBlock::Text {
        text: "wait".into(),
    }]
}

struct Held {
    arrived: oneshot::Receiver<()>,
    commit: oneshot::Sender<()>,
    drain: tokio::task::JoinHandle<Vec<Arc<EventRecord>>>,
}
fn hold_one_stop(mut receiver: mpsc::Receiver<JournalEntry>) -> Held {
    let (reached, arrived) = oneshot::channel();
    let (commit, release) = oneshot::channel();
    let drain = tokio::spawn(async move {
        let mut gate = Some((reached, release));
        let mut records = Vec::new();
        while let Some(item) = receiver.recv().await {
            match item {
                JournalEntry::Event(record) => records.push(record),
                JournalEntry::Checkpoint { change, reply } => {
                    if matches!(change, RestartCheckpoint::Suppress { .. })
                        && let Some((reached, release)) = gate.take()
                    {
                        reached.send(()).unwrap();
                        release.await.unwrap();
                    }
                    reply.send(Ok(true)).unwrap();
                }
            }
        }
        records
    });
    Held {
        arrived,
        commit,
        drain,
    }
}

async fn blocked_until_effect(action: SessionStop) {
    within(async {
        let mut fixture = Fixture::new();
        fixture.runtime = fixture.runtime.with_restart_journal();
        let Created { handle, durable } = fixture.runtime.create(fixture.spec()).await.unwrap();
        let Held {
            arrived,
            commit,
            drain,
        } = hold_one_stop(durable);
        until_running(&handle).await;
        let stopping = {
            let handle = handle.clone();
            tokio::spawn(async move { handle.stop(action).await })
        };
        arrived.await.unwrap();
        let mut prompt = Box::pin(handle.prompt(text(), Delivery::Start, Creator::Owner));
        std::future::poll_fn(|context| {
            assert!(prompt.as_mut().poll(context).is_pending());
            Poll::Ready(())
        })
        .await;
        // Rename has its own channel: the newer prompt reserves the sole
        // control slot before waiting for admission, so a control-lane
        // barrier would deadlock the fixture itself. The actor prioritizes
        // queued input before rename; an incorrectly admitted prompt waits
        // on Arm behind the held stop ACK and cannot reach this response.
        let title: SessionTitle = "stop checkpoint barrier".parse().unwrap();
        let renamed = tokio::time::timeout(RENAME_BARRIER, handle.rename(title.clone()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(renamed.title, Some(title));
        assert!(
            !handle
                .follow(EventSeq::ZERO)
                .unwrap()
                .cached
                .iter()
                .any(|record| {
                    matches!(
                        record.event,
                        SessionEvent::Agent {
                            event: AgentEvent::TurnStarted { .. }
                        }
                    )
                })
        );
        commit.send(()).unwrap();
        stopping.await.unwrap().unwrap();
        match action {
            SessionStop::Kill => assert!(matches!(prompt.await, Err(crate::SessionError::Ended))),
            SessionStop::Cancel { .. } => {
                assert_eq!(prompt.await.unwrap(), Delivered::Started);
                handle.kill();
            }
        }
        let _ended = handle.ended().await;
        fixture.runtime.shutdown().await;
        let records = drain.await.unwrap();
        let count = records
            .iter()
            .filter(|record| {
                matches!(
                    record.event,
                    SessionEvent::Agent {
                        event: AgentEvent::TurnStarted { .. },
                    }
                )
            })
            .count();
        assert_eq!(
            count,
            usize::from(matches!(action, SessionStop::Cancel { .. }))
        );
    })
    .await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_kill_commits_and_sets_its_signal_before_newer_input_can_rearm() {
    blocked_until_effect(SessionStop::Kill).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cancel_enqueues_its_effect_before_newer_input_can_rearm() {
    blocked_until_effect(SessionStop::Cancel { turn: None }).await;
}
