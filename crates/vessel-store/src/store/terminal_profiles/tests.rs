//! Profile settings use real committed CAS, protected persistence and safe metadata.

use super::*;
use crate::creation::tests::open;
use vessel_proto::terminal_profiles::{
    Argument, Id, Name, Profile, ProfileError, ProfileRevision, Profiles, Program,
};

#[tokio::test]
async fn absent_defaults_and_explicit_empty_remain_distinct_after_restart() {
    let (_temp, dir, store) = open().await;
    let defaults = store.terminal_profiles().await.unwrap();
    assert_eq!(defaults.revision, ProfileRevision::INITIAL);
    assert_eq!(defaults.profiles.entries().len(), 4);
    let disabled = store
        .set_terminal_profiles(SetTerminalProfiles {
            expected: defaults.revision,
            profiles: Profiles::new(Vec::new()).unwrap(),
        })
        .await
        .unwrap();
    assert!(disabled.profiles.entries().is_empty());
    assert_ne!(disabled.revision, defaults.revision);
    store.close().await;
    drop(store);
    let store = Store::open(&dir, crate::StoreConfig::default())
        .await
        .unwrap()
        .store;
    assert_eq!(store.terminal_profiles().await.unwrap(), disabled);
    store.close().await;
}

#[tokio::test]
async fn concurrent_last_revision_edit_has_one_winner_without_lost_configuration() {
    let (_temp, _dir, store) = open().await;
    let defaults = store.terminal_profiles().await.unwrap();
    let first = SetTerminalProfiles {
        expected: defaults.revision,
        profiles: Profiles::new(Vec::new()).unwrap(),
    };
    let second = first.clone();
    let (one, two) = tokio::join!(
        store.set_terminal_profiles(first),
        store.set_terminal_profiles(second)
    );
    assert_eq!(
        usize::from(one.is_ok()).saturating_add(usize::from(two.is_ok())),
        1
    );
    let failed = if let Err(error) = one {
        error
    } else {
        two.unwrap_err()
    };
    assert!(matches!(
        failed,
        StoreError::TerminalProfiles(ProfileError::StaleRevision)
    ));
    let current = store.terminal_profiles().await.unwrap();
    let unchanged = store
        .set_terminal_profiles(SetTerminalProfiles {
            expected: current.revision,
            profiles: current.profiles.clone(),
        })
        .await
        .unwrap();
    assert_eq!(current, unchanged);
    store.close().await;
}

#[tokio::test]
async fn picker_metadata_contains_no_program_or_argv_and_corrupt_settings_are_not_overwritten() {
    let (_temp, dir, store) = open().await;
    let profile = Profile::new(
        Id::parse("private-profile").unwrap(),
        Name::parse("Visible name").unwrap(),
        Program::parse("private-program-fixture").unwrap(),
        vec![Argument::parse("private-argument-fixture").unwrap()],
        None,
    )
    .unwrap();
    let settings = store
        .set_terminal_profiles(SetTerminalProfiles {
            expected: ProfileRevision::INITIAL,
            profiles: Profiles::new(vec![profile]).unwrap(),
        })
        .await
        .unwrap();
    let metadata = serde_json::to_string(&settings.summaries()).unwrap();
    assert!(metadata.contains("Visible name"));
    assert!(!metadata.contains("private-program-fixture"));
    assert!(!metadata.contains("private-argument-fixture"));
    store.close().await;
    drop(store);
    let db = rusqlite::Connection::open(dir.database_path()).unwrap();
    db.execute("UPDATE terminal_profile_settings SET profiles='{'", [])
        .unwrap();
    drop(db);
    let store = Store::open(&dir, crate::StoreConfig::default())
        .await
        .unwrap()
        .store;
    assert!(matches!(
        store.terminal_profiles().await,
        Err(StoreError::CorruptTerminalProfiles(_))
    ));
    assert!(
        store
            .set_terminal_profiles(SetTerminalProfiles {
                expected: settings.revision,
                profiles: Profiles::new(Vec::new()).unwrap()
            })
            .await
            .is_err()
    );
    store.close().await;
    let db = rusqlite::Connection::open(dir.database_path()).unwrap();
    let retained: String = db
        .query_row(
            "SELECT profiles FROM terminal_profile_settings",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(retained, "{");
}
