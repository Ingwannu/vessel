//! Strict disk decoding and CAS precedence, through the real `SQLite` schema.

use super::*;
use vessel_proto::launch_profiles::{LaunchChoices, LaunchProfile, LaunchSelection};
use vessel_proto::{HarnessKind, LaunchProfileId};

fn database() -> Connection {
    let mut connection = Connection::open_in_memory().unwrap();
    crate::schema::migrate(&mut connection).unwrap();
    connection
}
fn list(name: &str) -> LaunchProfiles {
    vec![LaunchProfile::new(
        LaunchProfileId::random(),
        name.parse().unwrap(),
        LaunchSelection::new(
            HarnessKind::ClaudeCode.into(),
            None,
            LaunchChoices::default(),
        ),
    )]
    .try_into()
    .unwrap()
}

#[test]
fn cas_checks_stale_before_equal_and_exhaustion_after_noop() {
    let connection = database();
    let profiles = list("first");
    let LaunchProfilesChange::Replaced(saved) =
        replace(&connection, LaunchProfileRevision::ZERO, &profiles).unwrap()
    else {
        panic!("not replaced")
    };
    assert_eq!(saved.revision().get(), 1);
    assert!(
        matches!(replace(&connection, LaunchProfileRevision::ZERO, &profiles).unwrap(), LaunchProfilesChange::Stale { current } if current == saved.revision())
    );
    assert!(
        matches!(replace(&connection, saved.revision(), &profiles).unwrap(), LaunchProfilesChange::Unchanged(current) if current == saved)
    );
    assert_eq!(
        connection
            .execute(
                "UPDATE launch_profiles SET revision = ?1",
                [i64::from(u32::MAX)],
            )
            .unwrap(),
        1
    );
    assert!(matches!(
        replace(&connection, LaunchProfileRevision::MAX, &profiles).unwrap(),
        LaunchProfilesChange::Unchanged(_)
    ));
    assert!(matches!(
        replace(&connection, LaunchProfileRevision::MAX, &list("different")).unwrap(),
        LaunchProfilesChange::Exhausted
    ));
    assert_eq!(read(&connection).unwrap().profiles(), &profiles);
}

#[test]
fn malformed_and_oversized_snapshots_never_become_empty_success() {
    let connection = database();
    // Unknown later record must fail the whole decode, retaining its source.
    let profiles = list("valid");
    let mut body = serde_json::to_string(&profiles).unwrap();
    assert_eq!(body.pop(), Some(']'));
    body.push_str(", {\"unexpected_private_text\": true}]");
    assert_eq!(
        connection
            .execute("UPDATE launch_profiles SET profiles = ?1", [&body])
            .unwrap(),
        1
    );
    let error = read(&connection).unwrap_err();
    assert!(matches!(error, StoreError::LaunchDecode(_)));
    assert!(!format!("{error:?}").contains("unexpected_private_text"));
    assert_eq!(
        connection
            .execute(
                "UPDATE launch_profiles SET profiles = ?1",
                ["x".repeat(LaunchPayload::MAX_BYTES + 1)],
            )
            .unwrap(),
        1
    );
    assert!(matches!(
        read(&connection),
        Err(StoreError::LaunchSnapshot(LaunchSnapshotProblem::TooLarge))
    ));
    assert_eq!(
        connection
            .execute(
                "UPDATE launch_profiles SET profiles = '[]', revision = -1",
                [],
            )
            .unwrap(),
        1
    );
    assert!(matches!(
        read(&connection),
        Err(StoreError::LaunchRevision { .. })
    ));
    assert_eq!(
        connection
            .execute("DELETE FROM launch_profiles", [])
            .unwrap(),
        1
    );
    assert!(matches!(
        read(&connection),
        Err(StoreError::LaunchSnapshot(LaunchSnapshotProblem::Missing))
    ));
}

#[test]
fn replacement_savepoint_rollback_keeps_complete_original_snapshot() {
    let mut connection = database();
    let original = read(&connection).unwrap();
    {
        let mut transaction = connection.transaction().unwrap();
        let savepoint = transaction.savepoint().unwrap();
        let changed = replace(&savepoint, original.revision(), &list("rolled back")).unwrap();
        assert!(matches!(changed, LaunchProfilesChange::Replaced(_)));
        drop(savepoint);
        transaction.commit().unwrap();
    }
    assert_eq!(read(&connection).unwrap(), original);
}
