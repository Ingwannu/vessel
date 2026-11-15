//! Passive reads share the real ordered writer and delayed-append watermark.
use vessel_proto::{
    AgentEvent, EventSeq, PermissionId, PermissionOutcome, PermissionRequest, SessionEvent,
    StopReason, ToolCallId,
};
use vessel_store::{StoreConfig, StoreError};

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
        store.read_attention(info.id, EventSeq::new(1), at(1)).await,
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
        .read_attention(info.id, EventSeq::new(3), at(2))
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
