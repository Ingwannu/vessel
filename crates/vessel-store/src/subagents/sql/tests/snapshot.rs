//! A real WAL snapshot, typed lease and durable detach, without an out-of-band read.
use super::queries;
use crate::{
    CredentialHash, StoredPush, devices,
    push::{self, PushWrite},
    schema,
};
use chrono::TimeDelta;
use rusqlite::Connection;
use vessel_proto::{
    DeviceId, DeviceInfo, DeviceSecret, Scope,
    push::{PushAuth, PushPublicKey, PushSubscription},
};
const LEASE: TimeDelta = TimeDelta::hours(1);
fn lease(db: &Connection) -> StoredPush {
    let id = DeviceId::random();
    devices::insert(
        db,
        &DeviceInfo {
            id,
            label: "parentage fixture".into(),
            scopes: vec![Scope::Read],
            created_at: queries::now(),
            last_seen: None,
            revoked_at: None,
        },
        &CredentialHash::of(&DeviceSecret::from_bytes([7; 32])),
    )
    .unwrap();
    let mut point = [0_u8; 65];
    point[0] = 4;
    let stored = StoredPush {
        device: id,
        subscription: PushSubscription {
            endpoint: "https://push.example/capability".parse().unwrap(),
            p256dh: PushPublicKey::from_bytes(&point).unwrap(),
            auth: PushAuth::from_bytes([9; 16]),
        },
        expires_at: queries::now().checked_add_signed(LEASE).unwrap(),
    };
    push::apply(
        db,
        &PushWrite::Subscribe {
            stored: Box::new(stored.clone()),
            now: queries::now(),
        },
    )
    .unwrap();
    stored
}
#[test]
fn source_and_final_lease_queries_use_their_statement_snapshot_and_current_receipt() {
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join("snapshot.sqlite");
    let mut writer = Connection::open(&path).unwrap();
    schema::configure_writer(&writer).unwrap();
    schema::migrate(&mut writer).unwrap();
    let source = queries::child(&writer, queries::parent());
    let lease = lease(&writer);
    assert!(!queries::current(&writer, source).unwrap());
    assert!(!queries::admitted(&writer, &lease, source).unwrap());
    let reader = Connection::open(&path).unwrap();
    schema::configure(&reader).unwrap();
    let before_detach = reader.unchecked_transaction().unwrap();
    assert!(queries::candidates(&before_detach, 1).unwrap().is_empty());
    queries::detach(&writer, source);
    assert!(queries::current(&writer, source).unwrap());
    assert!(queries::admitted(&writer, &lease, source).unwrap());
    assert!(!queries::current(&before_detach, source).unwrap());
    assert!(!queries::admitted(&before_detach, &lease, source).unwrap());
    before_detach.commit().unwrap();
    let transaction = reader.unchecked_transaction().unwrap();
    assert_eq!(
        queries::candidates(&transaction, 1).unwrap(),
        [source.key.session]
    );
    // A later writer may consume the attention and revoke its current lease.
    let changed = writer.unchecked_transaction().unwrap();
    changed
        .execute(
            "UPDATE inbox SET seen=1 WHERE session=?1",
            [source.key.session.to_string()],
        )
        .unwrap();
    devices::revoke(&changed, lease.device, queries::now()).unwrap();
    changed.commit().unwrap();
    assert!(queries::current(&transaction, source).unwrap());
    assert!(queries::admitted(&transaction, &lease, source).unwrap());
    transaction.commit().unwrap();
    assert!(queries::candidates(&reader, 1).unwrap().is_empty());
    assert!(!queries::current(&reader, source).unwrap());
    assert!(!queries::admitted(&reader, &lease, source).unwrap());
    // Marking the source unread again cannot resurrect the revoked lease.
    writer
        .execute(
            "UPDATE inbox SET seen=0 WHERE session=?1",
            [source.key.session.to_string()],
        )
        .unwrap();
    assert!(queries::current(&reader, source).unwrap());
    assert!(
        !queries::admitted(&reader, &lease, source).unwrap(),
        "revocation is independently retained"
    );
}
