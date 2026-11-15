//! Real writer acknowledgement, conflict, restart and failed-commit behavior.

use std::{future::Future, time::Duration};
use tempfile::TempDir;
use vessel_proto::launch_profiles::{
    LaunchChoices, LaunchProfile, LaunchProfileRevision, LaunchProfiles, LaunchSelection,
};
use vessel_proto::{HarnessKind, LaunchProfileId};
use vessel_store::{DataDir, LaunchProfilesChange, Store, StoreConfig, StoreError};

const LIMIT: Duration = Duration::from_secs(10);
async fn phase<T>(name: &str, future: impl Future<Output = T>) -> T {
    tokio::time::timeout(LIMIT, future)
        .await
        .unwrap_or_else(|_| panic!("phase timed out: {name}"))
}
async fn run(test: impl Future<Output = ()>) {
    vessel_testkit::within(Duration::from_secs(30), test)
        .await
        .unwrap();
}
async fn setup() -> (TempDir, DataDir, Store) {
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    dir.prepare().unwrap();
    let store = phase("open store", Store::open(&dir, StoreConfig::default()))
        .await
        .unwrap()
        .store;
    (temp, dir, store)
}
fn profile(name: &str) -> LaunchProfile {
    LaunchProfile::new(
        LaunchProfileId::random(),
        name.parse().unwrap(),
        LaunchSelection::new(HarnessKind::Codex.into(), None, LaunchChoices::default()),
    )
}

#[tokio::test]
async fn whole_replacement_preserves_order_clears_fields_and_survives_restart() {
    run(Box::pin(async {
        let (_temp, dir, store) = setup().await;
        let a = profile("a").with_notes(Some("old guidance".parse().unwrap()));
        let b = profile("b");
        let first: LaunchProfiles = vec![a.clone(), b.clone()].try_into().unwrap();
        let changed = phase(
            "save initial list",
            store.replace_launch_profiles(LaunchProfileRevision::ZERO, first),
        )
        .await
        .unwrap();
        let LaunchProfilesChange::Replaced(saved) = changed else {
            panic!("not replaced")
        };
        let replacement: LaunchProfiles = vec![b, a.with_notes(None)].try_into().unwrap();
        let changed = phase(
            "replace and reorder",
            store.replace_launch_profiles(saved.revision(), replacement.clone()),
        )
        .await
        .unwrap();
        let LaunchProfilesChange::Replaced(saved) = changed else {
            panic!("not replaced")
        };
        assert_eq!(saved.revision().get(), 2);
        assert_eq!(saved.profiles(), &replacement);
        phase("store close", store.close()).await;
        drop(store);
        let reopened = phase("reopen", Store::open(&dir, StoreConfig::default()))
            .await
            .unwrap()
            .store;
        assert_eq!(
            phase("read reopened snapshot", reopened.launch_profiles())
                .await
                .unwrap(),
            saved
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                std::fs::metadata(dir.database_path())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o077,
                0
            );
        }
        phase("close reopened", reopened.close()).await;
    }))
    .await;
}

#[tokio::test]
async fn two_writers_at_one_revision_have_exactly_one_winner() {
    run(Box::pin(async {
        let (_temp, _dir, store) = setup().await;
        let a: LaunchProfiles = vec![profile("a")].try_into().unwrap();
        let b: LaunchProfiles = vec![profile("b")].try_into().unwrap();
        let (left, right) = phase("concurrent CAS", async {
            tokio::join!(store.replace_launch_profiles(LaunchProfileRevision::ZERO, a), store.replace_launch_profiles(LaunchProfileRevision::ZERO, b))
        }).await;
        let answers = [left.unwrap(), right.unwrap()];
        assert_eq!(answers.iter().filter(|value| matches!(value, LaunchProfilesChange::Replaced(_))).count(), 1);
        assert_eq!(answers.iter().filter(|value| matches!(value, LaunchProfilesChange::Stale { current } if current.get() == 1)).count(), 1);
        assert_eq!(phase("committed winner", store.launch_profiles()).await.unwrap().revision().get(), 1);
        phase("close", store.close()).await;
    })).await;
}

#[tokio::test]
async fn failed_outer_commit_cannot_acknowledge_a_successful_savepoint() {
    run(Box::pin(async {
        let (_temp, dir, store) = setup().await;
        let connection = rusqlite::Connection::open(dir.database_path()).unwrap();
        // A deferred FK fails at the writer's outer COMMIT, after its
        // launch-profile savepoint itself has successfully released.
        connection.execute_batch("CREATE TABLE launch_test_parent (id INTEGER PRIMARY KEY);
          CREATE TABLE launch_test_child (parent INTEGER REFERENCES launch_test_parent(id) DEFERRABLE INITIALLY DEFERRED);
          CREATE TRIGGER launch_test_fail AFTER UPDATE ON launch_profiles BEGIN
            INSERT INTO launch_test_child (parent) VALUES (1);
          END;").unwrap();
        drop(connection);
        let next: LaunchProfiles = vec![profile("must roll back")].try_into().unwrap();
        let result = phase("failed commit acknowledgement", store.replace_launch_profiles(LaunchProfileRevision::ZERO, next)).await;
        assert!(matches!(result, Err(StoreError::Commit(_))));
        let kept = phase("read rollback", store.launch_profiles()).await.unwrap();
        assert_eq!(kept.revision(), LaunchProfileRevision::ZERO);
        assert!(kept.profiles().as_slice().is_empty());
        phase("close", store.close()).await;
    })).await;
}
