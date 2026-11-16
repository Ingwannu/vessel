//! Canonical typed decoding plus real SQL filtering before candidate limits.
use super::*;
use crate::{codec::encode, schema};
use chrono::DateTime;
use rusqlite::params;
use vessel_proto::{Creator, DeviceId, SessionId, SessionRef, subagents::DetachReceipt};
mod queries;
mod snapshot;

fn has_parent(
    db: &Connection,
    creator: Option<&str>,
    receipt: Option<&str>,
) -> rusqlite::Result<bool> {
    db.query_row(
        "SELECT vessel_has_current_parent(?1,?2)",
        params![creator, receipt],
        |r| r.get(0),
    )
}
#[test]
fn canonical_root_attached_and_detached_metadata_have_one_parentage_rule() {
    let db = queries::database();
    let parent = queries::parent();
    let creator = encode(&Creator::Agent {
        node: parent.node,
        session: parent.session,
    })
    .unwrap();
    let receipt = encode(&DetachReceipt::new(parent, DateTime::UNIX_EPOCH)).unwrap();
    assert!(!has_parent(&db, None, None).unwrap());
    assert!(!has_parent(&db, Some(&encode(&Creator::Owner).unwrap()), None).unwrap());
    assert!(
        !has_parent(
            &db,
            Some(
                &encode(&Creator::Device {
                    id: DeviceId::random()
                })
                .unwrap()
            ),
            None
        )
        .unwrap()
    );
    assert!(has_parent(&db, Some(&creator), None).unwrap());
    assert!(!has_parent(&db, Some(&creator), Some(&receipt)).unwrap());
}
#[test]
fn malformed_foreign_and_oversized_metadata_refuses_without_echoing_values() {
    let db = queries::database();
    let parent = queries::parent();
    let creator = encode(&Creator::Agent {
        node: parent.node,
        session: parent.session,
    })
    .unwrap();
    let foreign = SessionRef {
        node: queries::parent().node,
        session: parent.session,
    };
    let receipt = encode(&DetachReceipt::new(foreign, DateTime::UNIX_EPOCH)).unwrap();
    for (creator, receipt) in [
        (Some(creator.as_str()), Some(receipt.as_str())),
        (None, Some(receipt.as_str())),
        (Some("sensitive-invalid-origin"), None),
    ] {
        let error = has_parent(&db, creator, receipt).unwrap_err().to_string();
        assert!(error.contains("stored session parentage is invalid"));
        assert!(!error.contains("sensitive-invalid-origin"));
        assert!(!error.contains(&parent.node.to_string()));
    }
    let huge = "x".repeat(MAX_PARENTAGE_JSON_BYTES.saturating_add(1));
    assert!(has_parent(&db, Some(&huge), None).is_err());
    assert!(
        db.query_row::<bool, _, _>("SELECT vessel_has_current_parent(3,NULL)", [], |r| r.get(0))
            .is_err()
    );
}
#[test]
fn configured_reader_and_writer_have_the_scalar_and_a_missing_registration_fails_closed() {
    let db = Connection::open_in_memory().unwrap();
    assert!(has_parent(&db, None, None).is_err());
    schema::configure(&db).unwrap();
    assert!(!has_parent(&db, None, None).unwrap());
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join("parentage.sqlite");
    let writer = Connection::open(&path).unwrap();
    schema::configure_writer(&writer).unwrap();
    assert!(!has_parent(&writer, None, None).unwrap());
    let reader = crate::reader::Reader::new(path, crate::StoreConfig::default());
    assert!(!reader.with(|db| Ok(has_parent(db, None, None)?)).unwrap());
}
#[test]
fn detached_candidates_survive_more_than_one_page_of_attached_children() {
    const INTERNAL_PREFIX: usize = 65;
    const PAGE: usize = 1;
    let db = queries::database();
    let parent = queries::parent();
    for _ in 0..INTERNAL_PREFIX {
        let _attached = queries::child(&db, parent);
    }
    let detached = queries::child(&db, parent);
    assert!(queries::candidates(&db, PAGE).unwrap().is_empty());
    queries::detach(&db, detached);
    assert_eq!(
        queries::candidates(&db, PAGE).unwrap(),
        [detached.key.session]
    );
    assert!(queries::current(&db, detached).unwrap());
    let foreign = DetachReceipt::new(
        SessionRef {
            node: parent.node,
            session: SessionId::random(),
        },
        DateTime::UNIX_EPOCH,
    );
    let changed = db
        .execute(
            "UPDATE sessions SET detach_receipt=?2 WHERE id=?1",
            params![detached.key.session.to_string(), encode(&foreign).unwrap()],
        )
        .unwrap();
    assert_eq!(changed, 1);
    assert!(queries::candidates(&db, PAGE).is_err());
    assert!(queries::current(&db, detached).is_err());
}
