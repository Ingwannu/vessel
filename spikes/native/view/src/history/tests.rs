//! Regression tests use inert in-memory requests and replies, never an agent.

use super::*;
use crate::devices::MachineKey;
use crate::matcher::Query;
use std::path::PathBuf;
use vessel_proto::{HarnessKind, HarnessSessionId, SessionId, SessionMode, SessionTitle};

fn scope() -> Scope {
    Scope {
        context: crate::connection::ConnectionEpochs::default()
            .admit(
                MachineKey::from_book("laptop"),
                "00000000-0000-0000-0000-000000000001".parse().unwrap(),
            )
            .unwrap(),
        machine: MachineKey::from_book("laptop"),
        harness: HarnessKind::Codex.into(),
        profile: None,
        cwd: None,
    }
}
fn entry(id: &str) -> Entry {
    Entry {
        id: HarnessSessionId::new(id),
        cwd: PathBuf::from("/work/project"),
        title: Some(id.parse::<SessionTitle>().unwrap()),
        updated_at: None,
        in_vessel: None,
    }
}
fn page(ask: Ask, entries: Vec<Entry>, next: Option<Cursor>) -> Reply {
    match ask {
        Ask::List { scope, ticket, .. } => Reply::Listed {
            scope,
            ticket,
            result: Ok(Listed {
                entries,
                next,
                node: "00000000-0000-0000-0000-000000000001".parse().unwrap(),
            }),
        },
        Ask::Import(_) | Ask::Profiles { .. } => panic!("fake listing only accepts a listing"),
    }
}
fn loaded() -> History {
    let mut history = History::default();
    history.choose(scope());
    let ask = history.refresh().unwrap().unwrap();
    assert!(history.answer(page(ask, vec![entry("old"), entry("new")], None)));
    history
}

#[test]
fn the_fake_boundary_preserves_native_identity_and_directory() {
    let history = loaded();
    let scope = history.scope().unwrap();
    let entry = history.entries().first().unwrap();
    let spec = entry.spec(scope);
    assert_eq!(spec.cwd.as_ref(), Some(&entry.cwd));
    assert_eq!(spec.harness, scope.harness);
    assert_eq!(spec.profile, scope.profile);
    assert_eq!(spec.title, entry.title);
    assert!(
        matches!(spec.mode, SessionMode::Structured { resume: Some(id), .. } if id == entry.id)
    );
}

#[test]
fn closing_then_reopening_the_same_scope_rejects_the_old_page() {
    let mut h = History::default();
    h.choose(scope());
    let old = h.refresh().unwrap().unwrap();
    h.close();
    h.choose(scope());
    let fresh = h.refresh().unwrap().unwrap();
    assert!(!h.answer(page(old, vec![entry("wrong")], None)));
    assert!(h.answer(page(fresh, vec![entry("right")], None)));
    assert_eq!(h.entries().first().unwrap().id.as_str(), "right");
}

#[test]
fn machine_and_account_changes_reject_a_late_page() {
    let mut h = History::default();
    h.choose(scope());
    let old = h.refresh().unwrap().unwrap();
    let mut changed = scope();
    changed.machine = MachineKey::from_book("desktop");
    h.choose(changed);
    assert!(!h.answer(page(old, vec![entry("wrong")], None)));
    assert!(h.entries().is_empty());
}

#[test]
fn empty_pages_with_cursors_continue_and_duplicate_ids_replace() {
    let mut h = History::default();
    h.choose(scope());
    let first = h.refresh().unwrap().unwrap();
    assert!(h.answer(page(
        first,
        vec![],
        Some(Cursor::parse("opaque".into()).unwrap())
    )));
    assert!(h.has_more());
    let second = h.more().unwrap().unwrap();
    assert!(matches!(&second, Ask::List { cursor: Some(c), .. } if c.as_str() == "opaque"));
    assert!(h.answer(page(second, vec![entry("one"), entry("one")], None)));
    assert_eq!(h.entries().len(), 1);
    assert!(!h.has_more());
    assert!(h.more().unwrap().is_none());
}

#[test]
fn continuation_keeps_rows_but_refresh_replaces_and_clears_selection() {
    let mut h = loaded();
    h.toggle(&HarnessSessionId::new("old"));
    let ask = h.refresh().unwrap().unwrap();
    assert_eq!(h.entries().len(), 2);
    assert!(h.answer(page(ask, vec![entry("other")], None)));
    assert_eq!(h.entries().len(), 1);
    assert!(!h.has_selection());
}

#[test]
fn reported_times_sort_newest_first_with_unknown_dates_last() {
    let mut h = History::default();
    h.choose(scope());
    let mut older = entry("older");
    older.updated_at = chrono::DateTime::from_timestamp(1, 0);
    let mut newer = entry("newer");
    newer.updated_at = chrono::DateTime::from_timestamp(2, 0);
    let ask = h.refresh().unwrap().unwrap();
    assert!(h.answer(page(ask, vec![entry("unknown"), older, newer], None)));
    assert_eq!(
        h.entries()
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>(),
        ["newer", "older", "unknown"]
    );
}

#[test]
fn existing_rows_open_and_cannot_be_imported_again() {
    let mut h = History::default();
    h.choose(scope());
    let mut e = entry("existing");
    let id = SessionId::random();
    e.in_vessel = Some(id);
    let ask = h.refresh().unwrap().unwrap();
    assert!(h.answer(page(ask, vec![e], None)));
    h.toggle(&HarnessSessionId::new("existing"));
    assert_eq!(
        h.selection(&HarnessSessionId::new("existing")),
        Selection::Existing(id)
    );
    assert!(!h.has_selection());
    assert!(h.import().unwrap().is_none());
}

#[test]
fn multi_import_pins_scope_and_refuses_a_double_submission() {
    let mut h = loaded();
    for e in h.entries().to_vec() {
        h.toggle(&e.id);
    }
    let ask = h.import().unwrap().unwrap();
    assert!(h.import().unwrap().is_none());
    let Ask::Import(batch) = ask else {
        panic!("batch");
    };
    assert_eq!(batch.entries.len(), 2);
    let original = batch.scope.clone();
    let mut changed = scope();
    changed.cwd = Some(PathBuf::from("/other"));
    h.choose(changed);
    assert_eq!(h.scope(), Some(&original));
    assert!(h.answer(Ask::Import(batch).refused(Problem::Unsent)));
    assert!(!h.importing());
    assert!(!h.has_selection());
    assert_eq!(h.outcomes().len(), 2);
}

#[test]
fn uncertain_import_requires_reconciliation_before_repeat() {
    let mut h = loaded();
    h.toggle(&HarnessSessionId::new("new"));
    let ask = h.import().unwrap().unwrap();
    assert!(h.answer(ask.refused(Problem::Uncertain)));
    h.toggle(&HarnessSessionId::new("new"));
    assert!(h.import().unwrap().is_none());
    let read = h.refresh().unwrap().unwrap();
    assert!(h.answer(page(read, vec![entry("new")], None)));
    assert!(!h.requires_refresh());
    assert!(h.import().unwrap().is_some());
}

#[test]
fn search_matches_loaded_titles_and_paths_including_korean() {
    let h = loaded();
    assert_eq!(h.filtered(&Query::new("new")).len(), 1);
    assert_eq!(h.filtered(&Query::new("project")).len(), 2);
    assert!(h.filtered(&Query::new("없는제목")).is_empty());
}

#[test]
fn cursor_bound_is_checked_at_the_boundary() {
    assert!(Cursor::parse("x".repeat(Cursor::MAX_BYTES)).is_ok());
    assert_eq!(
        Cursor::parse("x".repeat(Cursor::MAX_BYTES.saturating_add(1))),
        Err(Problem::InvalidReply)
    );
}

#[test]
fn a_batch_receipt_must_match_scope_and_cover_every_selected_id_once() {
    let mut h = loaded();
    h.toggle(&HarnessSessionId::new("new"));
    let Ask::Import(batch) = h.import().unwrap().unwrap() else {
        panic!("batch");
    };
    let mut wrong = batch.scope.clone();
    wrong.machine = MachineKey::from_book("other");
    assert!(!h.answer(Reply::Imported {
        scope: wrong,
        ticket: batch.ticket,
        outcomes: vec![]
    }));
    assert!(h.importing());
    assert!(h.answer(Reply::Imported {
        scope: batch.scope,
        ticket: batch.ticket,
        outcomes: vec![]
    }));
    assert!(!h.importing());
    assert!(h.requires_refresh());
    assert_eq!(
        h.outcomes().first().unwrap().result.as_ref().unwrap_err(),
        &Problem::InvalidReply
    );
}

#[test]
fn successful_and_failed_imports_remain_attributed_to_their_rows() {
    let mut h = loaded();
    for entry in h.entries().to_vec() {
        h.toggle(&entry.id);
    }
    let Ask::Import(batch) = h.import().unwrap().unwrap() else {
        panic!("batch");
    };
    let first = batch.entries.first().unwrap();
    let created = SessionId::random();
    let outcomes = batch
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| Outcome {
            id: entry.id.clone(),
            result: if index == 0 {
                Ok(Box::new(vessel_proto::SessionInfo::new(
                    created,
                    entry.spec(&batch.scope),
                    chrono::DateTime::UNIX_EPOCH,
                )))
            } else {
                Err(Problem::Refused(vessel_proto::ErrorCode::Conflict))
            },
        })
        .collect();
    assert!(h.answer(Reply::Imported {
        scope: batch.scope,
        ticket: batch.ticket,
        outcomes
    }));
    assert_eq!(h.selection(&first.id), Selection::Existing(created));
    assert!(!h.importing());
    assert_eq!(h.outcomes().len(), batch.entries.len());
    assert!(!h.has_selection());
}

#[test]
fn a_new_authenticated_node_cannot_append_to_the_previous_nodes_page() {
    let mut h = History::default();
    h.choose(scope());
    let ask = h.refresh().unwrap().unwrap();
    assert!(h.answer(page(
        ask,
        vec![entry("original")],
        Some(Cursor::parse("next".into()).unwrap())
    )));
    let ask = h.more().unwrap().unwrap();
    let mut changed = page(ask, vec![entry("wrong-node")], None);
    if let Reply::Listed {
        result: Ok(page), ..
    } = &mut changed
    {
        page.node = vessel_proto::NodeId::random();
    }
    assert!(h.answer(changed));
    assert_eq!(h.loading(), &Loading::Failed(Problem::IdentityChanged));
    assert_eq!(h.entries().first().unwrap().id.as_str(), "original");
}

#[test]
fn existing_rows_reactivate_through_a_pinned_receipt_instead_of_a_create_selection() {
    let mut history = History::default();
    history.choose(scope());
    let ask = history.refresh().unwrap().unwrap();
    let mut existing = entry("old");
    let session = SessionId::random();
    existing.in_vessel = Some(session);
    assert!(history.answer(page(ask, vec![existing.clone()], None)));
    assert!(!history.has_selection());
    let Ask::Import(batch) = history.activate(&existing.id).unwrap().unwrap() else {
        panic!("activation batch")
    };
    assert_eq!(batch.entries, vec![existing]);
    assert_eq!(batch.entries.first().unwrap().in_vessel, Some(session));
    assert!(history.importing());
    assert!(
        history
            .activate(&HarnessSessionId::new("old"))
            .unwrap()
            .is_none()
    );
    assert!(history.answer(Reply::Imported {
        scope: batch.scope,
        ticket: batch.ticket,
        outcomes: vec![Outcome {
            id: HarnessSessionId::new("old"),
            result: Err(Problem::Uncertain)
        }]
    }));
    assert!(history.requires_refresh());
    assert!(
        history
            .activate(&HarnessSessionId::new("old"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn refusing_a_large_next_page_preserves_accepted_rows_selection_and_cursor() {
    const OVER_BUDGET: usize = 17 * 1024 * 1024;
    let mut history = History::default();
    history.choose(scope());
    let first = history.refresh().unwrap().unwrap();
    assert!(history.answer(page(
        first,
        vec![entry("retained")],
        Some(Cursor::parse("next".into()).unwrap()),
    )));
    let id = HarnessSessionId::new("retained");
    history.toggle(&id);
    let next = history.more().unwrap().unwrap();
    let mut oversized = entry("oversized");
    oversized.id = HarnessSessionId::new("x".repeat(OVER_BUDGET));
    assert!(history.answer(page(next, vec![oversized], None)));
    assert_eq!(history.loading(), &Loading::Failed(Problem::Capacity));
    assert_eq!(history.entries().len(), 1);
    assert_eq!(history.entries().first().unwrap().id, id);
    assert_eq!(history.selection(&id), Selection::Selected);
    assert!(history.has_more());
}
#[test]
fn oversized_import_selection_refuses_without_clearing_selected_rows() {
    let mut history = History::default();
    history.choose(scope());
    let rows = (0..=MAX_IMPORT_ENTRIES)
        .map(|index| entry(&format!("row-{index}")))
        .collect();
    let ask = history.refresh().unwrap().unwrap();
    assert!(history.answer(page(ask, rows, None)));
    let ids: Vec<_> = history.entries().iter().map(|row| row.id.clone()).collect();
    for id in &ids {
        history.toggle(id);
    }
    assert!(matches!(history.import(), Err(Problem::Capacity)));
    assert!(!history.importing());
    assert!(
        ids.iter()
            .all(|id| history.selection(id) == Selection::Selected)
    );
}
