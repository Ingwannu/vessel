//! Ordered writer commits reproduce the runtime publication/durability gap.
//! The producer holds an already-published record until the mark commits.
use std::sync::Arc;
use vessel_proto::inbox::{InboxCursor, InboxRead, ListInbox};
use vessel_proto::push::{PushAuth, PushPublicKey, PushSubscription};
use vessel_proto::{
    AgentEvent, DeviceId, DeviceInfo, DeviceSecret, EventRecord, EventSeq, Mark, PermissionId,
    PermissionRequest, Scope, SessionEvent, SessionId, SessionStatus, ToolCallId,
};
use vessel_store::{CredentialHash, PushAttention, Store, StoreConfig, StoredPush};

#[cfg(test)]
#[path = "support/fixtures.rs"]
#[expect(
    dead_code,
    reason = "shared builders; this suite uses only store and session fixtures"
)]
mod fixtures;
use fixtures::{at, data_dir, new_session, open, record};

#[cfg(test)]
mod support {
    use super::*;
    pub(super) async fn lease(store: &Store) -> StoredPush {
        let device = DeviceId::random();
        store
            .add_device(
                DeviceInfo {
                    id: device,
                    label: "push fixture".to_owned(),
                    scopes: vec![Scope::Read],
                    created_at: at(0),
                    last_seen: None,
                    revoked_at: None,
                },
                CredentialHash::of(&DeviceSecret::from_bytes([7; 32])),
            )
            .await
            .unwrap();
        let mut point = [7; 65];
        point[0] = 4;
        let lease = StoredPush {
            device,
            subscription: PushSubscription {
                endpoint: "https://push.example/capability".parse().unwrap(),
                p256dh: PushPublicKey::from_bytes(&point).unwrap(),
                auth: PushAuth::from_bytes([9; 16]),
            },
            expires_at: at(100),
        };
        store.subscribe_push(lease.clone(), at(0)).await.unwrap();
        lease
    }
    pub(super) fn request(seq: u64) -> Arc<EventRecord> {
        record(
            seq,
            SessionEvent::Agent {
                event: AgentEvent::PermissionRequested(PermissionRequest {
                    plan_proposal: None,
                    delivery_state: Default::default(),
                    tool_scope: None,
                    id: PermissionId::random(),
                    tool_call: Some(ToolCallId::new("fixture-tool")),
                    options: Vec::new(),

                    presentation: None,
                }),
            },
        )
    }
    pub(super) async fn delayed_append(
        store: Store,
        session: SessionId,
        records: Vec<Arc<EventRecord>>,
        mark: Mark,
    ) {
        let (release, released) = tokio::sync::oneshot::channel();
        let producer = store.clone();
        let drain = tokio::spawn(async move {
            released.await.unwrap();
            producer.append(session, records).await.unwrap();
        });
        let _marks = store.mark(session, mark, at(1)).await.unwrap();
        release.send(()).unwrap();
        drain.await.unwrap();
    }
}
use support::{delayed_append, lease, request};

#[tokio::test]
async fn archived_before_durable_drain_never_reappears_or_becomes_push_eligible() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let lease = lease(&store).await;
    let info = new_session(0);
    store.create_session(info.clone()).await.unwrap();
    store.append(info.id, vec![request(0)]).await.unwrap();
    let old = store
        .inbox(ListInbox::default())
        .await
        .unwrap()
        .entries
        .remove(0);
    delayed_append(
        store.clone(),
        info.id,
        vec![
            request(1),
            record(
                2,
                SessionEvent::Status {
                    status: SessionStatus::Exited { code: Some(0) },
                },
            ),
        ],
        Mark::Archive,
    )
    .await;
    let page = store.inbox(ListInbox::default()).await.unwrap();
    assert!(page.entries.is_empty());
    assert_eq!(page.counts.total.get(), 0);
    assert!(store.inbox_entry(old.key).await.unwrap().is_none());
    assert!(
        store
            .pending_push(InboxCursor::new(0), store.inbox_head().await.unwrap())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        !store
            .push_admitted(
                &lease,
                PushAttention {
                    key: old.key,
                    cursor: old.cursor
                },
                at(2)
            )
            .await
            .unwrap()
    );
    assert!(
        store
            .session(info.id)
            .await
            .unwrap()
            .unwrap()
            .marks
            .archived_at
            .is_some()
    );
    store.close().await;
}

#[tokio::test]
async fn read_before_durable_drain_stays_read_while_a_new_source_still_pushes() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let lease = lease(&store).await;
    let info = new_session(0);
    store.create_session(info.clone()).await.unwrap();
    delayed_append(
        store.clone(),
        info.id,
        vec![request(0), request(1), request(2)],
        Mark::Read {
            through: EventSeq::new(2),
        },
    )
    .await;
    let page = store.inbox(ListInbox::default()).await.unwrap();
    assert_eq!(page.counts.total.get(), 3);
    assert_eq!(page.counts.unread.get(), 1);
    let candidates = store
        .pending_push(InboxCursor::new(0), store.inbox_head().await.unwrap())
        .await
        .unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].key.seq, EventSeq::new(2));
    for entry in page.entries {
        let newer = entry.key.seq == EventSeq::new(2);
        assert_eq!(
            entry.read,
            if newer {
                InboxRead::Unread
            } else {
                InboxRead::Read
            }
        );
        assert_eq!(
            store
                .push_admitted(
                    &lease,
                    PushAttention {
                        key: entry.key,
                        cursor: entry.cursor
                    },
                    at(2)
                )
                .await
                .unwrap(),
            newer
        );
    }
    store.close().await;
}
