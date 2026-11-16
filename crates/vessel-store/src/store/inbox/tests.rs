//! Exercise the actual asynchronous Store consumers, not copies of their SQL.
//! Keeping these cases here reuses the private candidate-page policy without
//! publishing a test-only API or duplicating its boundary in fixtures.
use super::*;
use crate::{
    CredentialHash, DataDir, LocalAdmission, NotificationAuthority, PUSH_LEASE, StoreConfig,
    StoredPush,
};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use vessel_proto::inbox::{InboxCursor, InboxKey, InboxRead, ListInbox, MarkInbox};
use vessel_proto::push::{PushAuth, PushPublicKey, PushSubscription};
use vessel_proto::{
    AgentEvent, Creator, DeviceId, DeviceInfo, DeviceSecret, EventRecord, EventSeq, HarnessKind,
    NodeId, Scope, SessionEvent, SessionId, SessionInfo, SessionMode, SessionSpec, StopReason,
};

struct Fixture {
    // Drop pooled reader connections before removing their files on Windows.
    store: Store,
    _temp: tempfile::TempDir,
    now: DateTime<Utc>,
}
impl Fixture {
    async fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(&DataDir::new(temp.path()), StoreConfig::default())
            .await
            .unwrap()
            .store;
        Self {
            store,
            _temp: temp,
            now: Utc::now(),
        }
    }

    async fn finished(&self, creator: Creator) -> crate::PushAttention {
        let info = SessionInfo::new(
            SessionId::random(),
            SessionSpec::new(
                HarnessKind::Codex,
                SessionMode::Structured {
                    model: None,
                    resume: None,
                    options: Vec::new(),
                },
            ),
            self.now,
        )
        .with_creator(creator);
        let session = info.id;
        self.store.create_session(info).await.unwrap();
        self.store
            .append(
                session,
                vec![Arc::new(EventRecord {
                    seq: EventSeq::ZERO,
                    at: self.now,
                    event: SessionEvent::Agent {
                        event: AgentEvent::TurnEnded {
                            reason: StopReason::EndTurn,
                            usage: None,
                            point: None,
                        },
                    },
                })],
            )
            .await
            .unwrap();
        let entry = self
            .store
            .inbox_entry(InboxKey {
                session,
                seq: EventSeq::ZERO,
            })
            .await
            .unwrap()
            .unwrap();
        crate::PushAttention {
            key: entry.key,
            cursor: entry.cursor,
        }
    }

    async fn lease(&self) -> StoredPush {
        let device = DeviceId::random();
        self.store
            .add_device(
                DeviceInfo {
                    id: device,
                    label: "notification fixture".to_owned(),
                    scopes: vec![Scope::Read],
                    created_at: self.now,
                    last_seen: None,
                    revoked_at: None,
                },
                CredentialHash::of(&DeviceSecret::from_bytes([7; 32])),
            )
            .await
            .unwrap();
        let mut point = [0_u8; 65];
        point[0] = 4;
        let lease = StoredPush {
            device,
            subscription: PushSubscription {
                endpoint: "https://push.example/capability".parse().unwrap(),
                p256dh: PushPublicKey::from_bytes(&point).unwrap(),
                auth: PushAuth::from_bytes([9; 16]),
            },
            expires_at: self.now.checked_add_signed(PUSH_LEASE).unwrap(),
        };
        self.store
            .subscribe_push(lease.clone(), self.now)
            .await
            .unwrap();
        lease
    }
}

fn child() -> Creator {
    Creator::Agent {
        node: NodeId::random(),
        session: SessionId::random(),
    }
}

async fn assert_eligible(
    fixture: &Fixture,
    lease: &StoredPush,
    source: crate::PushAttention,
    expected: bool,
) {
    let candidates = fixture
        .store
        .pending_push(InboxCursor::new(0), source.cursor)
        .await
        .unwrap();
    assert_eq!(candidates, if expected { vec![source] } else { Vec::new() });
    assert_eq!(
        fixture
            .store
            .notification_current(NotificationAuthority::Owner, source)
            .await
            .unwrap(),
        expected
    );
    assert_eq!(
        fixture
            .store
            .push_admitted(lease, source, fixture.now)
            .await
            .unwrap(),
        expected
    );
}

#[tokio::test]
async fn detached_source_uses_actual_candidate_claim_and_final_lease_consumers() {
    let fixture = Fixture::new().await;
    let creator = child();
    let source = fixture.finished(creator.clone()).await;
    let lease = fixture.lease().await;
    assert_eligible(&fixture, &lease, source, false).await;
    assert_eq!(
        fixture
            .store
            .claim_notification(NotificationAuthority::Owner, source)
            .await
            .unwrap(),
        LocalAdmission::NoAttention
    );

    let detached = fixture
        .store
        .detach(source.key.session, fixture.now)
        .await
        .unwrap();
    assert_eq!(detached.session.created_by, creator);
    assert!(matches!(
        detached.session.parentage().unwrap(),
        vessel_proto::subagents::Parentage::Detached(_)
    ));
    assert_eligible(&fixture, &lease, source, true).await;
    assert_eq!(
        fixture
            .store
            .claim_notification(NotificationAuthority::Owner, source)
            .await
            .unwrap(),
        LocalAdmission::Admitted
    );

    fixture
        .store
        .mark_inbox(MarkInbox {
            key: source.key,
            read: InboxRead::Read,
        })
        .await
        .unwrap();
    assert_eligible(&fixture, &lease, source, false).await;
    fixture
        .store
        .mark_inbox(MarkInbox {
            key: source.key,
            read: InboxRead::Unread,
        })
        .await
        .unwrap();
    assert_eligible(&fixture, &lease, source, true).await;
    assert_eq!(
        fixture
            .store
            .claim_notification(NotificationAuthority::Owner, source)
            .await
            .unwrap(),
        LocalAdmission::AlreadyClaimed
    );

    fixture
        .store
        .revoke_device(lease.device, fixture.now)
        .await
        .unwrap();
    assert!(
        !fixture
            .store
            .push_admitted(&lease, source, fixture.now)
            .await
            .unwrap()
    );
    assert!(
        !fixture
            .store
            .notification_current(NotificationAuthority::Device(lease.device), source)
            .await
            .unwrap()
    );
    assert!(matches!(
        fixture
            .store
            .claim_notification(NotificationAuthority::Device(lease.device), source)
            .await,
        Err(StoreError::NotificationDenied)
    ));
    assert!(
        fixture
            .store
            .notification_current(NotificationAuthority::Owner, source)
            .await
            .unwrap()
    );
    fixture.store.close().await;
}

#[tokio::test]
async fn attached_pages_cannot_hide_a_later_root_or_detached_source() {
    for detached in [false, true] {
        let fixture = Fixture::new().await;
        for _ in 0..PUSH_CANDIDATE_PAGE.checked_add(1).unwrap() {
            let _attached = fixture.finished(child()).await;
        }
        let visible = fixture
            .finished(if detached { child() } else { Creator::Owner })
            .await;
        if detached {
            let _receipt = fixture
                .store
                .detach(visible.key.session, fixture.now)
                .await
                .unwrap();
        }
        assert_eq!(
            fixture
                .store
                .pending_push(
                    InboxCursor::new(0),
                    fixture.store.inbox_head().await.unwrap()
                )
                .await
                .unwrap(),
            vec![visible]
        );
        assert_eq!(
            fixture
                .store
                .inbox(ListInbox::default())
                .await
                .unwrap()
                .counts
                .total
                .get(),
            u64::try_from(PUSH_CANDIDATE_PAGE.checked_add(2).unwrap()).unwrap()
        );
        fixture.store.close().await;
    }
}
