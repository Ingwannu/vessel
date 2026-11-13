//! Real `SQLite` transaction boundaries for leases, revocation and stale 410s.
use super::*;
use crate::{CredentialHash, devices, schema};
use vessel_proto::{DeviceInfo, DeviceSecret};
fn now() -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000, 0).unwrap()
}
fn database() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    schema::configure(&db).unwrap();
    schema::migrate(&mut db).unwrap();
    db
}
fn device(db: &Connection, scopes: Vec<Scope>) -> DeviceId {
    let id = DeviceId::random();
    devices::insert(
        db,
        &DeviceInfo {
            id,
            label: "push fixture".to_owned(),
            scopes,
            created_at: now(),
            last_seen: None,
            revoked_at: None,
        },
        &CredentialHash::of(&DeviceSecret::from_bytes([7; 32])),
    )
    .unwrap();
    id
}
fn stored(id: DeviceId) -> StoredPush {
    let mut point = [0_u8; 65];
    point[0] = 4;
    StoredPush {
        device: id,
        subscription: PushSubscription {
            endpoint: "https://push.example/capability".parse().unwrap(),
            p256dh: vessel_proto::push::PushPublicKey::from_bytes(&point).unwrap(),
            auth: vessel_proto::push::PushAuth::from_bytes([9; 16]),
        },
        expires_at: now()
            .checked_add_signed(chrono::TimeDelta::hours(48))
            .unwrap(),
    }
}
#[test]
fn registration_requires_current_read_scope_and_revocation_cannot_resurrect() {
    let db = database();
    let denied = stored(device(&db, vec![Scope::Operate]));
    assert!(matches!(
        subscribe(&db, &denied, now()),
        Err(StoreError::PushDenied)
    ));
    let approved = stored(device(&db, vec![Scope::Read]));
    subscribe(&db, &approved, now()).unwrap();
    devices::revoke(&db, approved.device, now()).unwrap();
    assert!(list(&db, now()).unwrap().is_empty());
    assert!(matches!(
        subscribe(&db, &approved, now()),
        Err(StoreError::PushDenied)
    ));
}
#[test]
fn expired_lease_is_not_listed_and_late_410_cannot_delete_replacement() {
    let db = database();
    let old = stored(device(&db, vec![Scope::Read]));
    subscribe(&db, &old, now()).unwrap();
    assert!(list(&db, old.expires_at).unwrap().is_empty());
    let mut replacement = old.clone();
    replacement.subscription.endpoint = "https://push.example/replaced".parse().unwrap();
    subscribe(&db, &replacement, now()).unwrap();
    apply(
        &db,
        &PushWrite::Remove {
            device: old.device,
            expected: Some(old.subscription),
        },
    )
    .unwrap();
    assert_eq!(
        list(&db, now()).unwrap()[0].subscription,
        replacement.subscription
    );
    apply(
        &db,
        &PushWrite::Remove {
            device: replacement.device,
            expected: Some(replacement.subscription),
        },
    )
    .unwrap();
    assert!(list(&db, now()).unwrap().is_empty());
}
#[test]
fn sixty_four_subscriptions_allow_renewal_but_not_another_device_until_expiry() {
    let db = database();
    let mut leases = Vec::new();
    for _ in 0..64 {
        let lease = stored(device(&db, vec![Scope::Read]));
        subscribe(&db, &lease, now()).unwrap();
        leases.push(lease);
    }
    subscribe(&db, &leases[0], now()).unwrap();
    let mut extra = stored(device(&db, vec![Scope::Read]));
    assert!(matches!(
        subscribe(&db, &extra, now()),
        Err(StoreError::PushFull)
    ));
    let renewal = extra.expires_at;
    extra.expires_at = renewal
        .checked_add_signed(chrono::TimeDelta::hours(48))
        .unwrap();
    subscribe(&db, &extra, renewal).unwrap();
    assert_eq!(list(&db, now()).unwrap().len(), 1);
}
