//! Admission receipts use the actual writer, restart recovery and foreign-key
//! retention. No protocol fake can make a duplicate grant look successful.
use vessel_proto::inbox::{InboxRead, ListInbox, MarkInbox};
use vessel_proto::{
    AgentEvent, DeviceId, DeviceInfo, DeviceSecret, Scope, SessionEvent, SessionMode, StopReason,
};
use vessel_store::{
    CredentialHash, LocalAdmission, NotificationAuthority, PushAttention, StoreConfig, StoreError,
};

#[cfg(test)]
#[path = "support/fixtures.rs"]
#[expect(
    dead_code,
    reason = "shared session and temporary database builders; unrelated builders are unused"
)]
mod fixtures;
use fixtures::{at, data_dir, new_session, open, record};

#[tokio::test]
async fn simultaneous_admission_is_once_only_and_survives_restart() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let mut info = new_session(0);
    info.spec.mode = SessionMode::Structured {
        model: None,
        resume: None,
        options: Vec::new(),
    };
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
    let page = store.inbox(ListInbox::default()).await.unwrap();
    let entry = page.entries.first().unwrap();
    let source = PushAttention {
        key: entry.key,
        cursor: entry.cursor,
    };
    let (left, right) = tokio::join!(
        store.claim_notification(NotificationAuthority::Owner, source),
        store.claim_notification(NotificationAuthority::Owner, source)
    );
    let outcomes = [left.unwrap(), right.unwrap()];
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == LocalAdmission::Admitted)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == LocalAdmission::AlreadyClaimed)
            .count(),
        1
    );
    store.close().await;
    drop(store);
    let store = open(&dir, StoreConfig::default()).await;
    assert_eq!(
        store
            .claim_notification(NotificationAuthority::Owner, source)
            .await
            .unwrap(),
        LocalAdmission::AlreadyClaimed
    );
    store
        .mark_inbox(MarkInbox {
            key: source.key,
            read: InboxRead::Read,
        })
        .await
        .unwrap();
    assert!(
        !store
            .notification_current(NotificationAuthority::Owner, source)
            .await
            .unwrap()
    );
    store.close().await;
}

#[cfg(test)]
mod source_fixture {
    use super::*;
    pub(super) async fn permission_source(
        store: &vessel_store::Store,
    ) -> (
        vessel_proto::SessionId,
        vessel_proto::PermissionId,
        PushAttention,
    ) {
        let mut info = new_session(0);
        info.spec.mode = SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        };
        store.create_session(info.clone()).await.unwrap();
        let request = vessel_proto::PermissionRequest {
            id: vessel_proto::PermissionId::random(),
            tool_call: vessel_proto::ToolCallId::new("tool"),
            options: Vec::new(),
        };
        let request_id = request.id;
        store
            .append(
                info.id,
                vec![record(
                    0,
                    SessionEvent::Agent {
                        event: AgentEvent::PermissionRequested(request),
                    },
                )],
            )
            .await
            .unwrap();
        let entry = store
            .inbox(ListInbox::default())
            .await
            .unwrap()
            .entries
            .into_iter()
            .next()
            .unwrap();
        let source = PushAttention {
            key: entry.key,
            cursor: entry.cursor,
        };
        (info.id, request_id, source)
    }
}
use source_fixture::permission_source;
#[tokio::test]
async fn a_revoked_device_cannot_claim_the_owners_current_permission() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let (_session, _request, source) = permission_source(&store).await;
    let device = DeviceId::random();
    store
        .add_device(
            DeviceInfo {
                id: device,
                label: "claim test".to_owned(),
                scopes: vec![Scope::Read],
                created_at: at(0),
                last_seen: None,
                revoked_at: None,
            },
            CredentialHash::of(&DeviceSecret::from_bytes([7; 32])),
        )
        .await
        .unwrap();
    store.revoke_device(device, at(1)).await.unwrap();
    assert!(matches!(
        store
            .claim_notification(NotificationAuthority::Device(device), source)
            .await,
        Err(StoreError::NotificationDenied)
    ));
    assert_eq!(
        store
            .claim_notification(NotificationAuthority::Owner, source)
            .await
            .unwrap(),
        LocalAdmission::Admitted
    );
    store.close().await;
}
#[tokio::test]
async fn read_and_exact_resolution_after_commit_suppress_notification_delivery() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let (session, request_id, source) = permission_source(&store).await;
    assert_eq!(
        store
            .claim_notification(NotificationAuthority::Owner, source)
            .await
            .unwrap(),
        LocalAdmission::Admitted
    );
    store
        .mark_inbox(MarkInbox {
            key: source.key,
            read: InboxRead::Read,
        })
        .await
        .unwrap();
    assert!(
        !store
            .notification_current(NotificationAuthority::Owner, source)
            .await
            .unwrap()
    );
    store
        .mark_inbox(MarkInbox {
            key: source.key,
            read: InboxRead::Unread,
        })
        .await
        .unwrap();
    assert!(
        store
            .notification_current(NotificationAuthority::Owner, source)
            .await
            .unwrap()
    );
    store
        .append(
            session,
            vec![record(
                1,
                SessionEvent::Agent {
                    event: AgentEvent::PermissionResolved {
                        id: request_id,
                        outcome: vessel_proto::PermissionOutcome::Cancelled,
                    },
                },
            )],
        )
        .await
        .unwrap();
    assert!(
        !store
            .notification_current(NotificationAuthority::Owner, source)
            .await
            .unwrap()
    );
    assert_eq!(
        store
            .claim_notification(NotificationAuthority::Owner, source)
            .await
            .unwrap(),
        LocalAdmission::NoAttention
    );
    store.close().await;
}

#[tokio::test]
async fn failed_attention_can_claim_local_delivery_but_never_enters_web_push_candidates() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let mut info = new_session(0);
    info.spec.mode = SessionMode::Structured {
        model: None,
        resume: None,
        options: Vec::new(),
    };
    store.create_session(info.clone()).await.unwrap();
    store
        .append(
            info.id,
            vec![record(
                0,
                SessionEvent::Agent {
                    event: AgentEvent::TurnEnded {
                        reason: StopReason::Refusal,
                        usage: None,
                        point: None,
                    },
                },
            )],
        )
        .await
        .unwrap();
    let entry = store
        .inbox(ListInbox::default())
        .await
        .unwrap()
        .entries
        .into_iter()
        .next()
        .unwrap();
    assert!(matches!(entry.item, vessel_proto::inbox::InboxItem::Failed));
    let source = PushAttention {
        key: entry.key,
        cursor: entry.cursor,
    };
    assert!(
        store
            .pending_push(vessel_proto::inbox::InboxCursor::new(0), entry.cursor)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .notification_current(NotificationAuthority::Owner, source)
            .await
            .unwrap()
    );
    assert_eq!(
        store
            .claim_notification(NotificationAuthority::Owner, source)
            .await
            .unwrap(),
        LocalAdmission::Admitted
    );
    assert_eq!(
        store
            .claim_notification(NotificationAuthority::Owner, source)
            .await
            .unwrap(),
        LocalAdmission::AlreadyClaimed
    );
    store
        .mark_inbox(MarkInbox {
            key: source.key,
            read: InboxRead::Read,
        })
        .await
        .unwrap();
    assert!(
        !store
            .notification_current(NotificationAuthority::Owner, source)
            .await
            .unwrap()
    );
    store.close().await;
}
