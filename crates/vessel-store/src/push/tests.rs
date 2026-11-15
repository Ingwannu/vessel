//! Real `SQLite` transaction boundaries for leases, revocation and stale 410s.
use super::*;
use crate::{CredentialHash, devices, schema};
use vessel_proto::{DeviceInfo, DeviceSecret};
const LEASE_STEP: TimeDelta = TimeDelta::hours(1);
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
        expires_at: now().checked_add_signed(PUSH_LEASE).unwrap(),
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
    apply(&db, &PushWrite::Expire(Box::new(old))).unwrap();
    assert_eq!(
        list(&db, now()).unwrap()[0].subscription,
        replacement.subscription
    );
    apply(&db, &PushWrite::Expire(Box::new(replacement))).unwrap();
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
    extra.expires_at = renewal.checked_add_signed(PUSH_LEASE).unwrap();
    subscribe(&db, &extra, renewal).unwrap();
    assert_eq!(list(&db, now()).unwrap().len(), 1);
}

#[test]
fn late_gone_for_the_same_capability_cannot_delete_a_renewed_lease() {
    let db = database();
    let old = stored(device(&db, vec![Scope::Read]));
    subscribe(&db, &old, now()).unwrap();
    let mut renewed = old.clone();
    renewed.expires_at = old
        .expires_at
        .checked_add_signed(TimeDelta::seconds(1))
        .unwrap();
    subscribe(&db, &renewed, now()).unwrap();
    apply(&db, &PushWrite::Expire(Box::new(old))).unwrap();
    let retained = list(&db, now()).unwrap();
    assert_eq!(retained.len(), 1);
    assert_eq!(retained[0].expires_at, renewed.expires_at);
    assert_eq!(retained[0].subscription, renewed.subscription);
    apply(&db, &PushWrite::Expire(Box::new(renewed))).unwrap();
    assert!(list(&db, now()).unwrap().is_empty());
}

#[test]
fn explicit_unsubscribe_removes_a_renewal_without_an_attempt_identity() {
    let db = database();
    let lease = stored(device(&db, vec![Scope::Read]));
    subscribe(&db, &lease, now()).unwrap();
    apply(
        &db,
        &PushWrite::Remove {
            device: lease.device,
        },
    )
    .unwrap();
    assert!(list(&db, now()).unwrap().is_empty());
}

#[test]
fn activity_renews_at_half_life_and_stale_410_cannot_remove_the_renewal() {
    let db = database();
    let lease = stored(device(&db, vec![Scope::Read]));
    subscribe(&db, &lease, now()).unwrap();
    let half = now() + (PUSH_LEASE / 2);
    apply(
        &db,
        &PushWrite::Renew {
            device: lease.device,
            now: half - LEASE_STEP,
        },
    )
    .unwrap();
    assert_eq!(list(&db, now()).unwrap()[0].expires_at, lease.expires_at);
    apply(
        &db,
        &PushWrite::Renew {
            device: lease.device,
            now: half,
        },
    )
    .unwrap();
    assert_eq!(list(&db, half).unwrap()[0].expires_at, half + PUSH_LEASE);
    let later = half + LEASE_STEP;
    apply(
        &db,
        &PushWrite::Renew {
            device: lease.device,
            now: later,
        },
    )
    .unwrap();
    assert_eq!(list(&db, later).unwrap()[0].expires_at, half + PUSH_LEASE);
    apply(&db, &PushWrite::Expire(Box::new(lease))).unwrap();
    assert_eq!(list(&db, later).unwrap().len(), 1);
}
#[test]
fn activity_cannot_resurrect_revoked_readless_or_expired_push_leases() {
    for state in 0..3 {
        let db = database();
        let lease = stored(device(&db, vec![Scope::Read]));
        subscribe(&db, &lease, now()).unwrap();
        let later = if state == 2 {
            now() + (PUSH_LEASE + LEASE_STEP)
        } else {
            now() + (PUSH_LEASE / 2 + LEASE_STEP)
        };
        match state {
            0 => devices::revoke(&db, lease.device, now()).unwrap(),
            1 => {
                let _changed = db
                    .execute(
                        "UPDATE devices SET scopes='[]' WHERE id=?1",
                        [lease.device.to_string()],
                    )
                    .unwrap();
            }
            _ => {}
        }
        apply(
            &db,
            &PushWrite::Renew {
                device: lease.device,
                now: later,
            },
        )
        .unwrap();
        let expiry: Option<i64> = db
            .query_row(
                "SELECT expires_at FROM push_subscriptions WHERE device=?1",
                [lease.device.to_string()],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        if state == 0 {
            // The existing revoke transaction deletes this subscription.
            // Activity must leave it absent, rather than recreating a row.
            assert!(expiry.is_none());
        } else {
            assert_eq!(expiry.map(time_from_sql), Some(lease.expires_at));
        }
        assert!(list(&db, later).unwrap().is_empty());
    }
}

#[test]
fn lease_only_admission_reuses_exact_expiry_revocation_scope_and_missing_device_rules() {
    let db = database();
    let id = device(&db, vec![Scope::Read]);
    let old = stored(id);
    subscribe(&db, &old, now()).unwrap();
    assert!(lease_admitted(&db, &old, now()).unwrap());
    assert!(!lease_admitted(&db, &old, old.expires_at).unwrap());
    let mut renewed = old.clone();
    renewed.expires_at += LEASE_STEP;
    subscribe(&db, &renewed, now()).unwrap();
    assert!(!lease_admitted(&db, &old, now()).unwrap());
    assert!(lease_admitted(&db, &renewed, now()).unwrap());
    let _changed = db
        .execute(
            "UPDATE devices SET scopes='[\"operate\"]' WHERE id=?1",
            [id.to_string()],
        )
        .unwrap();
    assert!(!lease_admitted(&db, &renewed, now()).unwrap());
    let _changed = db
        .execute(
            "UPDATE devices SET scopes='[\"read\"]',revoked_at=1 WHERE id=?1",
            [id.to_string()],
        )
        .unwrap();
    assert!(!lease_admitted(&db, &renewed, now()).unwrap());
    let _changed = db
        .execute("DELETE FROM devices WHERE id=?1", [id.to_string()])
        .unwrap();
    assert!(!lease_admitted(&db, &renewed, now()).unwrap());
}
