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

#[tokio::test]
async fn revocation_before_a_queued_claim_and_read_before_confirmation_suppress_delivery() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
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
            info.id,
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
