//! Actual serial file outcomes drive bounded delivery; optimistic values do not.

use super::super::{Coordinator, Outcome};
use super::*;
use crate::Prefs;
use std::{path::PathBuf, time::Duration};
use vessel_app_view::prefs::{Kept, Theme};

const DEADLINE: Duration = Duration::from_secs(10);

struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _cleanup = std::fs::remove_dir_all(&self.0);
    }
}

fn writable() -> (Temp, PathBuf, Prefs, Coordinator) {
    let root = Temp(std::env::temp_dir().join(format!(
        "vessel-saved-effects-{}",
        vessel_proto::SessionId::random()
    )));
    let path = root.0.join("prefs.json");
    let initial = Prefs {
        kept: Kept::File {
            path: path.display().to_string(),
        },
        ..Prefs::default()
    };
    initial.save_at(&path).unwrap();
    let coordinator = Coordinator::start(initial.clone()).unwrap();
    (root, path, initial, coordinator)
}

async fn publish(client: &super::super::Client, prefs: &Prefs) -> Snapshot {
    let receipt = client.publish_with_receipt(prefs).unwrap();
    tokio::time::timeout(DEADLINE, receipt.completed())
        .await
        .unwrap()
        .unwrap()
}

fn desired(initial: &Prefs, limit: u32) -> Prefs {
    let mut next = initial.clone();
    next.settings.terminal_scrollback = ScrollbackLines::new(limit).unwrap();
    next
}

#[tokio::test]
async fn slow_clients_preserve_saved_minimum_final_and_late_clients_receive_no_old_floor() {
    let (_root, path, initial, coordinator) = writable();
    let (first, _first_watch) = coordinator.client();
    let (second, _second_watch) = coordinator.client();
    assert_eq!(
        first.drain_scrollback().unwrap().unwrap().minimum(),
        ScrollbackLines::DEFAULT
    );
    assert!(second.drain_scrollback().unwrap().is_some());
    let zero = publish(&first, &desired(&initial, 0)).await;
    let eight = publish(&second, &desired(&initial, 8)).await;
    assert!(matches!(zero.outcome(), Outcome::Saved));
    assert!(matches!(eight.outcome(), Outcome::Saved));
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let mut unrelated = (*coordinator.current()).clone();
    unrelated.settings.appearance.theme = Theme::Light;
    let failed = publish(&second, &unrelated).await;
    assert!(matches!(failed.outcome(), Outcome::Failed(_)));
    assert_eq!(failed.committed_scrollback(), eight.committed_scrollback());
    for client in [&first, &second] {
        let batch = client.drain_scrollback().unwrap().unwrap();
        assert_eq!(batch.minimum().get(), 0);
        assert_eq!(batch.latest().limit().get(), 8);
        assert_eq!(batch.latest().revision(), eight.revision());
        assert_eq!(client.applied_scrollback().get(), 8);
        assert!(client.drain_scrollback().unwrap().is_none());
    }
    let (late, _late_watch) = coordinator.client();
    let batch = late.drain_scrollback().unwrap().unwrap();
    assert_eq!(
        batch.minimum().get(),
        8,
        "new windows do not replay another owner's shrink"
    );
    assert_eq!(batch.latest(), eight.committed_scrollback().unwrap());
}

#[tokio::test]
async fn a_failed_shrink_never_enters_the_later_successful_batch() {
    let (_root, path, initial, coordinator) = writable();
    let (client, _watch) = coordinator.client();
    assert!(client.drain_scrollback().unwrap().is_some());
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let failed = publish(&client, &desired(&initial, 0)).await;
    assert!(matches!(failed.outcome(), Outcome::Failed(_)));
    assert!(client.drain_scrollback().unwrap().is_none());
    assert_eq!(client.applied_scrollback(), ScrollbackLines::DEFAULT);
    std::fs::remove_dir(&path).unwrap();
    let saved = publish(&client, &desired(&initial, 8)).await;
    assert!(matches!(saved.outcome(), Outcome::Saved));
    let batch = client.drain_scrollback().unwrap().unwrap();
    assert_eq!(batch.minimum().get(), 8);
    assert_eq!(batch.latest().limit().get(), 8);
    let disk: Prefs = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(disk.settings.terminal_scrollback.get(), 8);
}

#[tokio::test]
async fn launch_only_projects_values_without_trimming_or_advancing_new_clients() {
    let initial = Prefs::default();
    let coordinator = Coordinator::start(initial.clone()).unwrap();
    let (client, _watch) = coordinator.client();
    assert!(client.drain_scrollback().unwrap().is_none());
    for limit in [0, 8] {
        let snapshot = publish(&client, &desired(&initial, limit)).await;
        assert!(matches!(snapshot.outcome(), Outcome::LaunchOnly));
        assert!(snapshot.committed_scrollback().is_none());
        assert!(client.drain_scrollback().unwrap().is_none());
        assert_eq!(client.applied_scrollback(), ScrollbackLines::DEFAULT);
    }
    let (late, _watch) = coordinator.client();
    assert!(late.drain_scrollback().unwrap().is_none());
    assert_eq!(late.applied_scrollback(), ScrollbackLines::DEFAULT);
}

#[tokio::test]
async fn unavailable_delivery_refuses_effects_publication_and_lifecycle_currentness() {
    let (_root, _path, initial, coordinator) = writable();
    let (client, _watch) = coordinator.client();
    assert!(client.drain_scrollback().unwrap().is_some());
    let checkpoint = tokio::time::timeout(DEADLINE, coordinator.checkpoint())
        .await
        .unwrap()
        .unwrap();
    let hub = coordinator.scrollback.clone();
    let poisoned = std::thread::spawn(move || {
        let _held = hub.0.slots.lock().unwrap();
        panic!("injected delivery poison");
    })
    .join();
    assert!(poisoned.is_err());
    assert!(!coordinator.is_current(&checkpoint));
    assert!(matches!(
        client.drain_scrollback(),
        Err(DesktopError::Closed)
    ));
    assert_eq!(client.publish(&initial), Err(crate::Rejected::Closed));
    assert!(
        tokio::time::timeout(DEADLINE, coordinator.checkpoint())
            .await
            .unwrap()
            .is_err()
    );
}

#[tokio::test]
async fn dropped_clients_are_pruned_and_each_late_subscriber_gets_the_current_record() {
    let (_root, _path, initial, coordinator) = writable();
    let (writer, _watch) = coordinator.client();
    let (gone, _gone_watch) = coordinator.client();
    drop(gone);
    let saved = publish(&writer, &desired(&initial, 8)).await;
    let (late, _late_watch) = coordinator.client();
    assert_eq!(coordinator.scrollback.0.slots.lock().unwrap().len(), 2);
    let batch = late.drain_scrollback().unwrap().unwrap();
    assert_eq!(batch.latest(), saved.committed_scrollback().unwrap());
    assert_eq!(batch.minimum().get(), 8);
}
