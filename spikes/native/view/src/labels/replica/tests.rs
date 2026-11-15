//! Resync, disconnect and atomic-batch failures preserve cached colors.

use super::*;
use crate::labels::Color;
use vessel_proto::{WorkspaceLabelGeneration, WorkspaceLabelSeq};

fn label(name: &str, color: Color) -> Label {
    Label {
        name: name.parse().unwrap(),
        color,
    }
}
fn cursor() -> CatalogCursor {
    CatalogCursor {
        generation: WorkspaceLabelGeneration::random(),
        after: WorkspaceLabelSeq::ZERO,
    }
}
fn initial(replica: &mut Replica, head: CatalogCursor) {
    replica
        .listed(&CatalogList {
            head,
            data: CatalogMode::Snapshot {
                labels: vec![label("Ready", Color::Violet)],
            },
        })
        .unwrap();
}

#[test]
fn offline_and_failed_first_fetch_are_not_authoritative_deletions() {
    let mut replica = Replica::default();
    replica.failed(Failure::Load);
    assert_ne!(replica.state(), HostState::Online);
    let head = cursor();
    initial(&mut replica, head);
    replica.disconnected();
    assert_eq!(replica.cursor(), Some(head));
    assert_eq!(
        replica.catalog().get(&"ready".parse().unwrap()),
        Some(&label("Ready", Color::Violet))
    );
    replica.reconnecting();
    replica.failed(Failure::Load);
    assert_eq!(replica.catalog().labels().count(), 1);
    assert!(replica.state().writable().is_ok());
    assert_ne!(replica.state(), HostState::Online);
}

#[test]
fn a_contiguous_update_recovers_a_connected_load_failure_with_a_known_cursor() {
    let mut replica = Replica::default();
    let head = cursor();
    initial(&mut replica, head);
    replica.failed(Failure::Load);
    let next = CatalogCursor {
        after: head.after.next().unwrap(),
        ..head
    };
    replica
        .updated(&CatalogUpdate {
            cursor: next,
            delta: CatalogDelta::Upsert {
                label: label("Ready", Color::Sky),
                previous: None,
            },
        })
        .unwrap();
    assert_eq!(replica.state(), HostState::Online);
    assert_eq!(replica.cursor(), Some(next));
    assert_eq!(
        replica.catalog().get(&"ready".parse().unwrap()),
        Some(&label("Ready", Color::Sky))
    );
}

#[test]
fn gap_or_foreign_generation_preserves_cache_and_requires_snapshot() {
    for next in [WorkspaceLabelSeq::ZERO, WorkspaceLabelSeq::new(2)] {
        let mut replica = Replica::default();
        let head = cursor();
        initial(&mut replica, head);
        let update = CatalogUpdate {
            cursor: CatalogCursor {
                after: next,
                ..head
            },
            delta: CatalogDelta::Remove {
                name: "Ready".parse().unwrap(),
            },
        };
        assert_eq!(replica.updated(&update), Err(Failure::Load));
        assert_eq!(replica.cursor(), None);
        assert_eq!(replica.catalog().labels().count(), 1);
    }
    let mut replica = Replica::default();
    initial(&mut replica, cursor());
    assert_eq!(
        replica.updated(&CatalogUpdate {
            cursor: CatalogCursor {
                after: WorkspaceLabelSeq::new(1),
                ..cursor()
            },
            delta: CatalogDelta::Remove {
                name: "Ready".parse().unwrap()
            }
        }),
        Err(Failure::Load)
    );
}

#[test]
fn malformed_catch_up_is_atomic_and_snapshot_removes_missing_definitions() {
    let mut replica = Replica::default();
    let head = cursor();
    initial(&mut replica, head);
    let next = CatalogCursor {
        after: WorkspaceLabelSeq::new(1),
        ..head
    };
    let renamed = CatalogUpdate {
        cursor: next,
        delta: CatalogDelta::Upsert {
            label: label("Waiting", Color::Sky),
            previous: Some("Ready".parse().unwrap()),
        },
    };
    let invalid = CatalogList {
        head: CatalogCursor {
            after: WorkspaceLabelSeq::new(2),
            ..head
        },
        data: CatalogMode::Changes {
            from: head,
            updates: vec![renamed.clone()],
        },
    };
    assert_eq!(replica.listed(&invalid), Err(Failure::Load));
    assert_eq!(
        replica.catalog().get(&"Ready".parse().unwrap()),
        Some(&label("Ready", Color::Violet))
    );
    initial(&mut replica, head);
    replica
        .listed(&CatalogList {
            head: next,
            data: CatalogMode::Changes {
                from: head,
                updates: vec![renamed],
            },
        })
        .unwrap();
    assert!(replica.catalog().get(&"Ready".parse().unwrap()).is_none());
    assert_eq!(
        replica.catalog().get(&"Waiting".parse().unwrap()),
        Some(&label("Waiting", Color::Sky))
    );
    replica
        .listed(&CatalogList {
            head: cursor(),
            data: CatalogMode::Snapshot { labels: Vec::new() },
        })
        .unwrap();
    assert_eq!(replica.catalog().labels().count(), 0);
}

#[test]
fn duplicate_snapshot_or_wrong_base_cannot_replace_an_existing_projection() {
    let mut replica = Replica::default();
    initial(&mut replica, cursor());
    assert_eq!(
        replica.listed(&CatalogList {
            head: cursor(),
            data: CatalogMode::Snapshot {
                labels: vec![label("Same", Color::Sky), label("same", Color::Red)]
            }
        }),
        Err(Failure::Load)
    );
    assert_eq!(
        replica.catalog().get(&"Ready".parse().unwrap()),
        Some(&label("Ready", Color::Violet))
    );
    let base = cursor();
    assert_eq!(
        replica.listed(&CatalogList {
            head: base,
            data: CatalogMode::Changes {
                from: base,
                updates: Vec::new()
            }
        }),
        Err(Failure::Load)
    );
}
