//! Paired devices in the store (ADR 0010).

use fixtures::{at, device, store};
use vessel_proto::{DeviceId, DeviceSecret};
use vessel_store::{CredentialHash, Store, StoreConfig, StoreError};

#[cfg(test)]
#[path = "support/registry_rollback.rs"]
mod registry_rollback;

#[test]
fn cancelled_window_retains_admission_while_its_blocking_read_is_queued() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let (_temp, _dir, store) = store().await;
        let slots = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
        let permit = std::sync::Arc::clone(&slots).try_acquire_owned().unwrap();
        let (entered, started) = tokio::sync::oneshot::channel();
        let (release, until_released) = std::sync::mpsc::sync_channel(1);
        let blocker = tokio::task::spawn_blocking(move || {
            entered.send(()).unwrap();
            until_released.recv().unwrap();
        });
        started.await.unwrap();
        let mut pending = Box::pin(store.guarded_device_window(None, permit));
        assert!(futures_util::poll!(&mut pending).is_pending());
        drop(pending);
        assert_eq!(
            slots.available_permits(),
            0,
            "queued disk work must still own admission"
        );
        release.send(()).unwrap();
        blocker.await.unwrap();
        let returned = tokio::time::timeout(std::time::Duration::from_secs(30), slots.acquire())
            .await
            .expect("the detached bounded read never released admission")
            .unwrap();
        drop(returned);
        store.close().await;
    });
}

#[tokio::test]
async fn device_windows_keep_a_stable_cursor_and_explicit_incompleteness() {
    let (_temp, _dir, store) = store().await;
    let mut expected = Vec::new();
    for index in 0..=vessel_store::DeviceWindow::MAX_ITEMS {
        let info = device("window fixture", i64::from(index));
        store
            .add_device(
                info.clone(),
                CredentialHash::of(&DeviceSecret::from_bytes([8; 32])),
            )
            .await
            .unwrap();
        expected.push(info);
    }
    expected.sort_by_key(|info| info.id);
    let first = store.device_window(None).await.unwrap();
    assert!(first.more);
    let last = first.devices.last().unwrap().id;
    let second = store.device_window(Some(last)).await.unwrap();
    assert!(!second.more);
    assert_eq!(second.devices.len(), 1);
    let last = second.devices.last().unwrap().id;
    assert_eq!([first.devices, second.devices].concat(), expected);
    let empty = store.device_window(Some(last)).await.unwrap();
    assert!(!empty.more);
    assert!(empty.devices.is_empty());
    store.close().await;
}

/// Undoes schemas 16 to 7, so a test can take a database further back:
/// the inbox and push subscriptions (16), the usage ledger (15), the
/// bounded side plan (14), search (13), schedules, saved prompts and hooks
/// (12), pull request feedback (10; 11's
/// column goes with `approvals`), the workspace's tables (9), retention's
/// index (schema 1's in its place, 8), the conversation's columns and the
/// attachments table (7).
const UNDO_SCHEMAS_7_TO_16: &str = "ALTER TABLE sessions DROP COLUMN detach_receipt;
     ALTER TABLE sessions DROP COLUMN title_origin;
     DROP TABLE usage_samples;
     DROP TABLE usage_meta;
     ALTER TABLE sessions DROP COLUMN usage_cost_baseline;
     ALTER TABLE sessions DROP COLUMN latest_plan;
     DROP TRIGGER sessions_search_gone;
     DROP TRIGGER sessions_search_pruned;
     DROP TABLE search_index;
     DROP TABLE search_messages;
     DROP TABLE search_gone;
     DROP INDEX sessions_search_behind;
     ALTER TABLE sessions DROP COLUMN search_open;
     ALTER TABLE sessions DROP COLUMN search_seq;
DROP TABLE inbox_tools;
     DROP TABLE inbox;
     DROP TABLE inbox_meta;
     DROP TABLE push_subscriptions;
     DROP TABLE hook_keys;
     DROP TABLE hook_deliveries;
     DROP TABLE hooks;
     DROP TABLE saved_prompts;
     DROP TABLE schedule_runs;
     DROP TABLE schedules;
     DROP TABLE forge_feedback_sent;
     DROP TABLE forge_feedback_on;
     DROP TABLE approvals;
     DROP TABLE worktrees;
     DROP TABLE projects;
     DROP INDEX sessions_retention;
     CREATE INDEX sessions_ended ON sessions (ended_at) WHERE live = 0;
     ALTER TABLE sessions DROP COLUMN abilities;
     ALTER TABLE sessions DROP COLUMN usage;
     ALTER TABLE sessions DROP COLUMN queue;
     ALTER TABLE sessions DROP COLUMN attention_seq;
     ALTER TABLE sessions DROP COLUMN read_seq;
     ALTER TABLE sessions DROP COLUMN marked_unread;
     ALTER TABLE sessions DROP COLUMN pinned;
     ALTER TABLE sessions DROP COLUMN archived_at;
     DROP TABLE attachments;";

/// Builders. A `cfg(test)` module so the lints' test allowances (`unwrap`
/// and friends) apply to it.
#[cfg(test)]
mod fixtures {
    use chrono::{DateTime, Utc};
    use tempfile::TempDir;
    use vessel_proto::{DeviceId, DeviceInfo, Scope};
    use vessel_store::{DataDir, Store, StoreConfig};

    pub(crate) fn at(second: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000_i64.saturating_add(second), 0).unwrap()
    }

    pub(crate) fn device(label: &str, created: i64) -> DeviceInfo {
        DeviceInfo {
            id: DeviceId::random(),
            label: label.to_owned(),
            created_at: at(created),
            last_seen: None,
            scopes: Scope::ALL.to_vec(),
            revoked_at: None,
        }
    }

    pub(crate) async fn store() -> (TempDir, DataDir, Store) {
        let temp = tempfile::tempdir().unwrap();
        let dir = DataDir::new(temp.path());
        dir.prepare().unwrap();
        let store = Store::open(&dir, StoreConfig::default())
            .await
            .unwrap()
            .store;
        (temp, dir, store)
    }
}

#[tokio::test]
async fn devices_are_stored_with_their_credential_hash_only() {
    let (_temp, _dir, store) = store().await;
    let secret = DeviceSecret::from_bytes([4; 32]);
    let laptop = device("laptop", 0);
    let phone = device("phone", 1);
    store
        .add_device(laptop.clone(), CredentialHash::of(&secret))
        .await
        .unwrap();
    store
        .add_device(
            phone.clone(),
            CredentialHash::of(&DeviceSecret::from_bytes([5; 32])),
        )
        .await
        .unwrap();

    assert_eq!(store.devices().await.unwrap(), [laptop.clone(), phone]);
    let found = store.device(laptop.id).await.unwrap().unwrap();
    assert_eq!(found.info, laptop);
    assert!(found.credential.matches(&secret));
    assert!(!found.credential.matches(&DeviceSecret::from_bytes([5; 32])));
    assert!(store.device(DeviceId::random()).await.unwrap().is_none());

    assert!(matches!(
        store
            .add_device(laptop.clone(), CredentialHash::of(&secret))
            .await,
        Err(StoreError::DeviceExists(id)) if id == laptop.id
    ));
}

#[tokio::test]
async fn revoking_keeps_a_tombstone_and_its_first_time() {
    let (_temp, dir, store) = store().await;
    let laptop = device("laptop", 0);
    let secret = DeviceSecret::from_bytes([4; 32]);
    store
        .add_device(laptop.clone(), CredentialHash::of(&secret))
        .await
        .unwrap();
    store.device_seen(laptop.id, at(5)).await.unwrap();
    store.revoke_device(laptop.id, at(10)).await.unwrap();
    store.revoke_device(laptop.id, at(20)).await.unwrap();

    let found = store.device(laptop.id).await.unwrap().unwrap();
    assert_eq!(found.info.last_seen, Some(at(5)));
    assert_eq!(found.info.revoked_at, Some(at(10)));
    assert!(
        found.credential.matches(&secret),
        "kept to answer `Revoked`"
    );

    let unknown = DeviceId::random();
    assert!(matches!(
        store.revoke_device(unknown, at(30)).await,
        Err(StoreError::UnknownDevice(id)) if id == unknown
    ));

    // Survives a restart.
    store.close().await;
    drop(store);
    let reopened = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    assert_eq!(
        reopened.devices().await.unwrap()[0].revoked_at,
        Some(at(10))
    );
}

/// A database from milestone 2 (schema version 1) gains the devices table
/// on open, and keeps its sessions.
#[tokio::test]
async fn a_version_1_database_gains_the_devices_table() {
    let (_temp, dir, store) = store().await;
    store.close().await;
    drop(store);
    let connection = rusqlite::Connection::open(dir.database_path()).unwrap();
    connection
        .execute_batch("ALTER TABLE sessions DROP COLUMN listing;")
        .unwrap();
    connection
        .execute_batch("ALTER TABLE sessions DROP COLUMN labels;")
        .unwrap();
    registry_rollback::undo_registry_schema(&connection);
    connection.execute_batch(UNDO_SCHEMAS_7_TO_16).unwrap();
    // Back to schema 1: without the devices table (2), the sessions'
    // profile and creator columns (3), where they came from (4) and their
    // model (5).
    connection
        .execute_batch(
            "DROP TABLE devices;
             ALTER TABLE sessions DROP COLUMN profile;
             ALTER TABLE sessions DROP COLUMN created_by;
             ALTER TABLE sessions DROP COLUMN via;
             ALTER TABLE sessions DROP COLUMN origin;
             ALTER TABLE sessions DROP COLUMN model;
             ALTER TABLE sessions DROP COLUMN title;
             ALTER TABLE sessions DROP COLUMN choices;",
        )
        .unwrap();
    connection.pragma_update(None, "user_version", 1).unwrap();
    drop(connection);

    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    assert!(store.devices().await.unwrap().is_empty());
    let laptop = device("laptop", 0);
    store
        .add_device(
            laptop,
            CredentialHash::of(&DeviceSecret::from_bytes([1; 32])),
        )
        .await
        .unwrap();
}
