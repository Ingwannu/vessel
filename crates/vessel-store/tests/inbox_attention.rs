//! Passive reads share the real ordered writer and delayed-append watermark.
use vessel_proto::{
    AgentEvent, EventSeq, PermissionId, PermissionOutcome, PermissionRequest, SessionEvent,
    StopReason, ToolCallId,
};
use vessel_store::{Residence, StoreConfig, StoreError};

#[cfg(test)]
#[path = "support/fixtures.rs"]
#[expect(dead_code, reason = "shared temporary database and session builders")]
mod fixtures;
use fixtures::{at, data_dir, new_session, open, record};

#[tokio::test]
async fn passive_read_vetoes_waiting_and_delayed_inserts_inherit_only_observed_watermark() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let info = new_session(0);
    store.create_session(info.clone()).await.unwrap();
    let request = PermissionRequest {
        id: PermissionId::random(),
        tool_call: ToolCallId::new("tool"),
        options: Vec::new(),
    };
    store
        .append(
            info.id,
            vec![record(
                0,
                SessionEvent::Agent {
                    event: AgentEvent::PermissionRequested(request.clone()),
                },
            )],
        )
        .await
        .unwrap();
    assert!(matches!(
        store
            .read_attention(info.id, EventSeq::new(1), at(1), Residence::Stored)
            .await,
        Err(StoreError::AttentionWaiting)
    ));
    store
        .append(
            info.id,
            vec![record(
                1,
                SessionEvent::Agent {
                    event: AgentEvent::PermissionResolved {
                        id: request.id,
                        outcome: PermissionOutcome::Cancelled,
                    },
                },
            )],
        )
        .await
        .unwrap();
    let _kept = store
        .read_attention(info.id, EventSeq::new(3), at(2), Residence::Stored)
        .await
        .unwrap();
    for seq in 2..4 {
        store
            .append(
                info.id,
                vec![record(
                    seq,
                    SessionEvent::Agent {
                        event: AgentEvent::TurnEnded {
                            reason: StopReason::EndTurn,
                            usage: None,
                            point: None,
                        },
                    },
                )],
            )
            .await
            .unwrap();
    }
    let attention = store.inbox_attention(vec![info.id]).await.unwrap();
    assert_eq!(attention[0].unread.get(), 1);
    assert_eq!(attention[0].waiting.get(), 0);
    store.close().await;
}

#[tokio::test]
async fn two_readers_observe_the_deliberate_flag_and_entry_counts_as_one_snapshot() {
    const CHANGES: usize = 32;
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let info = new_session(0);
    store.create_session(info.clone()).await.unwrap();
    store
        .append(
            info.id,
            vec![record(
                0,
                SessionEvent::Agent {
                    event: AgentEvent::TurnEnded {
                        reason: StopReason::EndTurn,
                        usage: None,
                        point: None,
                    },
                },
            )],
        )
        .await
        .unwrap();
    let mark = vessel_proto::Mark::Read {
        through: EventSeq::new(1),
    };
    let _kept = store.mark(info.id, mark, at(1)).await.unwrap();
    let writing = store.clone();
    let writer = tokio::spawn(async move {
        for _change in 0..CHANGES {
            let _kept = writing
                .mark(info.id, vessel_proto::Mark::Unread, at(2))
                .await
                .unwrap();
            let _kept = writing.mark(info.id, mark, at(3)).await.unwrap();
        }
    });
    for _sample in 0..CHANGES {
        let (first, second) = tokio::join!(
            store.inbox_attention(vec![info.id]),
            store.inbox_attention(vec![info.id])
        );
        for state in [first.unwrap(), second.unwrap()] {
            assert_eq!(state.len(), 1);
            assert_eq!(state[0].explicit_unread, state[0].unread.get() == 1);
        }
    }
    writer.await.unwrap();
    store.close().await;
}

/// One `inbox.attention` call reads every session it names from one
/// snapshot (owed item 5 of the node review of TRACKER 815: the test above
/// names one session). The writer adds a waiting request to the first
/// session, then to the second, so in any one snapshot the first has as
/// many as the second or one more. Reads made one session at a time could
/// see the second ahead of the first.
#[tokio::test]
async fn two_sessions_are_read_from_one_snapshot() {
    const CHANGES: u64 = 32;
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let first = new_session(0);
    let second = new_session(1);
    for info in [&first, &second] {
        store.create_session(info.clone()).await.unwrap();
    }
    let writing = store.clone();
    let (one, two) = (first.id, second.id);
    let writer = tokio::spawn(async move {
        for seq in 0..CHANGES {
            for session in [one, two] {
                let request = PermissionRequest {
                    id: PermissionId::random(),
                    tool_call: ToolCallId::new("tool"),
                    options: Vec::new(),
                };
                writing
                    .append(
                        session,
                        vec![record(
                            seq,
                            SessionEvent::Agent {
                                event: AgentEvent::PermissionRequested(request),
                            },
                        )],
                    )
                    .await
                    .unwrap();
            }
        }
    });
    let mut samples = 0_u32;
    while !writer.is_finished() || samples == 0 {
        for order in [vec![one, two], vec![two, one]] {
            let state = store.inbox_attention(order).await.unwrap();
            let waiting = |session| {
                state
                    .iter()
                    .find(|row| row.session == session)
                    .map_or(0, |row| row.waiting.get())
            };
            let (ahead, behind) = (waiting(one), waiting(two));
            assert!(
                ahead == behind || ahead == behind.saturating_add(1),
                "one snapshot: the first session has {ahead} waiting, the second {behind}"
            );
        }
        samples = samples.saturating_add(1);
    }
    writer.await.unwrap();
    let last = store.inbox_attention(vec![one, two]).await.unwrap();
    assert!(last.iter().all(|row| row.waiting.get() == CHANGES));
    store.close().await;
}
