//! Canonical ordering, exact receipts and refusal recovery through one writer.

use super::*;
use vessel_app_view::prefs::{Grouping, Theme};

#[tokio::test]
async fn full_queue_refuses_later_snapshots_until_the_window_observes_canonical_state() {
    let initial = Prefs::default();
    let coordinator = Coordinator::start(initial.clone()).unwrap();
    let (client, _updates) = coordinator.client();
    for _ in 0..QUEUE_CAPACITY {
        client.publish(&initial).unwrap();
    }
    let mut refused = initial.clone();
    refused.settings.appearance.theme = Theme::Light;
    assert_eq!(client.publish(&refused), Err(crate::Rejected::Full));
    coordinator.flush().await.unwrap();
    let canonical = coordinator.snapshot();
    // Even though the physical queue has drained, the UI still holds a refused
    // field. A later unrelated snapshot must not silently retry that field.
    let mut later = refused;
    later.settings.sidebar.group = Grouping::Status;
    assert_eq!(client.publish(&later), Err(crate::Rejected::Full));
    assert_eq!(coordinator.snapshot().revision(), canonical.revision());
    assert_eq!(*canonical.preferences(), initial);
    let Observation::Applied(reconciled) = client.observe(&canonical) else {
        panic!("current canonical state");
    };
    let mut fresh = (*reconciled).clone();
    fresh.settings.sidebar.group = Grouping::Status;
    client.publish(&fresh).unwrap();
    coordinator.flush().await.unwrap();
    let persisted = coordinator.current();
    assert_eq!(
        persisted.settings.appearance.theme,
        initial.settings.appearance.theme
    );
    assert_eq!(persisted.settings.sidebar.group, Grouping::Status);
}

#[tokio::test]
async fn exact_receipt_keeps_its_failed_outcome_after_a_later_successful_write() {
    let root = std::env::temp_dir().join(format!(
        "vessel-pref-receipt-{}",
        vessel_proto::SessionId::random()
    ));
    let path = root.join("prefs.json");
    std::fs::create_dir_all(&path).unwrap();
    let initial = Prefs {
        kept: Kept::File {
            path: path.display().to_string(),
        },
        ..Prefs::default()
    };
    let coordinator = Coordinator::start(initial.clone()).unwrap();
    let (first, _first_updates) = coordinator.client();
    let (second, _second_updates) = coordinator.client();
    let mut rejected = initial.clone();
    rejected.settings.appearance.theme = Theme::Light;
    let failure = first.publish_with_receipt(&rejected).unwrap();
    coordinator.flush().await.unwrap();
    std::fs::remove_dir(&path).unwrap();
    let mut accepted = initial;
    accepted.settings.sidebar.group = Grouping::Status;
    let success = second.publish_with_receipt(&accepted).unwrap();
    coordinator.flush().await.unwrap();
    let newer = success.completed().await.unwrap();
    let older = failure.completed().await.unwrap();
    assert!(matches!(newer.outcome(), Outcome::Saved));
    assert!(matches!(older.outcome(), Outcome::Failed(_)));
    assert!(older.revision() < newer.revision());
    assert_eq!(newer.preferences().settings.sidebar.group, Grouping::Status);
    let Observation::Applied(projected) = first.observe(&newer) else {
        panic!("canonical projection");
    };
    assert_eq!(projected, newer.preferences());
    assert!(matches!(first.observe(&older), Observation::Stale));
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn older_ack_keeps_pending_intent_and_old_failure_cannot_roll_back_newer_commit() {
    let initial = Prefs::default();
    let coordinator = Coordinator::start(initial.clone()).unwrap();
    let (client, _updates) = coordinator.client();
    let original_canonical = client.canonical_preferences();
    let mut first = initial;
    first.settings.appearance.theme = Theme::Light;
    client.publish(&first).unwrap();
    assert_eq!(*client.canonical_preferences(), *original_canonical);
    coordinator.flush().await.unwrap();
    let older = coordinator.snapshot();
    let mut next = first;
    next.settings.sidebar.group = Grouping::Status;
    client.publish(&next).unwrap();
    // This explicit older snapshot remains deterministic even if the writer
    // has already acknowledged the newer request on the other thread.
    let Observation::Applied(projected) = client.observe(&older) else {
        panic!("valid projection");
    };
    assert_eq!(projected.settings.sidebar.group, Grouping::Status);
    assert_eq!(*client.canonical_preferences(), *older.preferences());
    coordinator.flush().await.unwrap();
    let Observation::Applied(committed) = client.observe(&coordinator.snapshot()) else {
        panic!("current projection");
    };
    assert_eq!(committed.settings.sidebar.group, Grouping::Status);
    assert_eq!(*client.canonical_preferences(), *coordinator.current());
    let mut old_failure = older;
    old_failure.outcome = Outcome::Failed(Arc::new(DesktopError::Closed));
    assert!(matches!(client.observe(&old_failure), Observation::Stale));
    assert_eq!(
        client.baseline.borrow().settings.sidebar.group,
        Grouping::Status
    );
}

#[tokio::test]
async fn stale_windows_merge_and_failed_save_keeps_canonical_state() {
    let root = std::env::temp_dir().join(format!("vessel-central-prefs-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let initial = Prefs {
        kept: Kept::File {
            path: root.join("prefs.json").display().to_string(),
        },
        ..Prefs::default()
    };
    let coordinator = Coordinator::start(initial.clone()).unwrap();
    let (first, _updates) = coordinator.client();
    let (second, mut updates) = coordinator.client();
    let mut one = initial.clone();
    one.settings.appearance.theme = Theme::Light;
    first.publish(&one).unwrap();
    coordinator.flush().await.unwrap();
    let mut two = initial;
    two.settings.sidebar.group = Grouping::Status;
    second.publish(&two).unwrap();
    coordinator.flush().await.unwrap();
    updates.changed().await.unwrap();
    assert_eq!(
        coordinator.current().settings.appearance.theme,
        Theme::Light
    );
    assert_eq!(
        coordinator.current().settings.sidebar.group,
        Grouping::Status
    );
    std::fs::remove_file(root.join("prefs.json")).unwrap();
    std::fs::create_dir(root.join("prefs.json")).unwrap();
    let committed = coordinator.current();
    let mut refused = (*committed).clone();
    refused.manual_updates = true;
    second.publish(&refused).unwrap();
    coordinator.flush().await.unwrap();
    assert_eq!(coordinator.current(), committed);
    assert!(matches!(updates.borrow().outcome(), Outcome::Failed(_)));
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn corrupt_file_remains_launch_only_and_never_gets_replaced() {
    let root = std::env::temp_dir().join(format!("vessel-corrupt-prefs-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("prefs.json");
    std::fs::write(&path, b"unreadable").unwrap();
    let initial = Prefs {
        kept: Kept::ThisLaunch {
            why: vessel_app_view::prefs::LaunchOnly::Unreadable {
                fault: vessel_app_view::files::FileFault {
                    path: path.display().to_string(),
                    at: None,
                    problem: vessel_app_view::files::FileProblem::CannotOpen,
                    field: None,
                },
            },
        },
        ..Prefs::default()
    };
    let coordinator = Coordinator::start(initial.clone()).unwrap();
    let (client, updates) = coordinator.client();
    let mut next = initial;
    next.manual_updates = true;
    client.publish(&next).unwrap();
    coordinator.flush().await.unwrap();
    assert!(coordinator.current().manual_updates);
    assert!(matches!(updates.borrow().outcome(), Outcome::LaunchOnly));
    assert_eq!(std::fs::read(path).unwrap(), b"unreadable");
    std::fs::remove_dir_all(root).unwrap();
}
