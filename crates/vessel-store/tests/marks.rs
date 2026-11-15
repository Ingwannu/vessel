//! A person's marks on a session, deleting one, and the order retention
//! prunes ended sessions in (protocol 10, ADR 0026).

use std::sync::Arc;

use fixtures::{at, data_dir, new_session, open, record, status};
use vessel_proto::{
    AgentEvent, EventRecord, EventSeq, KeptMarks, Mark, SessionBoard, SessionEvent, SessionId,
    SessionMarks, SessionStatus, StopReason,
};
use vessel_store::{DataDir, Retention, Store, StoreConfig, StoreError};

#[cfg(test)]
#[path = "support/fixtures.rs"]
#[expect(
    dead_code,
    reason = "builders shared with the log's tests; this file uses some"
)]
mod fixtures;

/// Builders for this file. A `cfg(test)` module so the lints' test
/// allowances apply.
#[cfg(test)]
mod support {
    use super::*;

    pub(crate) fn turn_ended(seq: u64) -> Arc<EventRecord> {
        record(
            seq,
            SessionEvent::Agent {
                event: AgentEvent::TurnEnded {
                    reason: StopReason::EndTurn,
                    usage: None,
                    point: None,
                },
            },
        )
    }

    /// A stored session, started and running: records 0 and 1.
    pub(crate) async fn running(store: &Store, created: i64) -> SessionId {
        let session = new_session(created);
        store.create_session(session.clone()).await.unwrap();
        store
            .append(
                session.id,
                vec![
                    status(0, SessionStatus::Starting),
                    status(1, SessionStatus::Running),
                ],
            )
            .await
            .unwrap();
        session.id
    }

    /// End `session`, whose next record is `seq`, at `second`.
    pub(crate) async fn end(store: &Store, session: SessionId, seq: u64, second: i64) {
        let end = Arc::new(EventRecord {
            seq: EventSeq::new(seq),
            at: at(second),
            event: SessionEvent::Status {
                status: SessionStatus::Exited { code: Some(0) },
            },
        });
        store.append(session, vec![end]).await.unwrap();
    }

    pub(crate) async fn marks(store: &Store, session: SessionId) -> SessionMarks {
        store.session(session).await.unwrap().unwrap().marks
    }

    /// Give `session` an attachment folder holding one file.
    pub(crate) fn attach(dir: &DataDir, session: SessionId) -> std::path::PathBuf {
        let folder = dir.attachments_dir().join(session.to_string());
        std::fs::create_dir_all(folder.join("some-id")).unwrap();
        std::fs::write(folder.join("some-id").join("notes.txt"), b"x").unwrap();
        folder
    }

    pub(crate) async fn stored(store: &Store) -> Vec<SessionId> {
        let mut ids: Vec<SessionId> = store
            .sessions()
            .await
            .unwrap()
            .into_iter()
            .map(|info| info.id)
            .collect();
        ids.sort();
        ids
    }

    pub(crate) fn sorted(mut ids: Vec<SessionId>) -> Vec<SessionId> {
        ids.sort();
        ids
    }
}

use support::{attach, end, marks, running, sorted, stored, turn_ended};

#[tokio::test]
async fn each_mark_is_kept_and_survives_a_reopen() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let id = running(&store, 0).await;
    assert_eq!(marks(&store, id).await, SessionMarks::default());

    let pinned = store.mark(id, Mark::Pin, at(10)).await.unwrap();
    assert!(pinned.pinned);
    let archived = store.mark(id, Mark::Archive, at(11)).await.unwrap();
    assert_eq!(archived.archived_at, Some(at(11)));
    let again = store.mark(id, Mark::Archive, at(12)).await.unwrap();
    assert_eq!(again.archived_at, Some(at(11)), "the first time stays");
    let unread = store.mark(id, Mark::Unread, at(13)).await.unwrap();
    assert!(unread.marked_unread);
    assert_eq!(
        marks(&store, id).await,
        SessionMarks {
            pinned: true,
            archived_at: Some(at(11)),
            unread: true,
        }
    );

    store.close().await;
    drop(store);
    let store = open(&dir, StoreConfig::default()).await;
    assert!(marks(&store, id).await.pinned, "kept by the store");
    let unpinned = store.mark(id, Mark::Unpin, at(14)).await.unwrap();
    let unarchived = store.mark(id, Mark::Unarchive, at(15)).await.unwrap();
    assert_eq!(
        (unpinned.pinned, unarchived.archived_at),
        (false, None),
        "taken back"
    );
    assert!(matches!(
        store.mark(SessionId::random(), Mark::Pin, at(16)).await,
        Err(StoreError::UnknownSession(_))
    ));
}

/// `read` only moves forward, and clears `unread` whatever its `through`.
#[tokio::test]
async fn read_only_moves_forward() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let id = running(&store, 0).await;
    let read = |through| Mark::Read {
        through: EventSeq::new(through),
    };
    let kept = store.mark(id, read(2), at(10)).await.unwrap();
    assert_eq!(kept.read, Some(EventSeq::new(2)));
    let behind = store.mark(id, read(1), at(11)).await.unwrap();
    assert_eq!(
        behind.read,
        Some(EventSeq::new(2)),
        "an older read changes nothing"
    );
    let _kept = store.mark(id, Mark::Unread, at(12)).await.unwrap();
    let cleared = store.mark(id, read(0), at(13)).await.unwrap();
    assert_eq!(
        cleared,
        KeptMarks {
            read: Some(EventSeq::new(2)),
            ..KeptMarks::default()
        },
        "any read ends a marked unread"
    );
}

/// Unread compares positions: a turn's end and a `read` agree in either
/// order (the contract's "Unread, without a race").
#[tokio::test]
async fn unread_follows_the_log_and_the_last_read_in_either_order() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;

    // The turn ends first, then a read that saw it.
    let first = running(&store, 0).await;
    assert!(!marks(&store, first).await.unread, "nothing asked yet");
    store.append(first, vec![turn_ended(2)]).await.unwrap();
    assert!(marks(&store, first).await.unread, "never read");
    let through = EventSeq::new(3);
    let _kept = store
        .mark(first, Mark::Read { through }, at(10))
        .await
        .unwrap();
    assert!(!marks(&store, first).await.unread, "seen");

    // A read first, then a turn that ends at the point it named: unread.
    let second = running(&store, 1).await;
    let _kept = store
        .mark(
            second,
            Mark::Read {
                through: EventSeq::new(2),
            },
            at(10),
        )
        .await
        .unwrap();
    assert!(!marks(&store, second).await.unread);
    store.append(second, vec![turn_ended(2)]).await.unwrap();
    assert!(marks(&store, second).await.unread, "after the read's point");

    // The session's end asks for attention too.
    end(&store, first, 3, 20).await;
    assert!(marks(&store, first).await.unread, "the end, not yet read");
}

/// `session.delete` removes an ended session whole, folder included; a live
/// one is refused, and so is an unknown one.
#[tokio::test]
async fn an_ended_session_is_deleted_whole() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let ended = running(&store, 0).await;
    end(&store, ended, 2, 5).await;
    let live = running(&store, 1).await;
    let folder = attach(&dir, ended);
    let kept = attach(&dir, live);

    assert!(matches!(
        store.delete_session(live).await,
        Err(StoreError::SessionLive(id)) if id == live
    ));
    store.delete_session(ended).await.unwrap();
    assert_eq!(store.session(ended).await.unwrap(), None);
    assert!(matches!(
        store.read_events(ended, EventSeq::ZERO, 10).await,
        Err(StoreError::UnknownSession(_))
    ));
    assert!(!folder.exists(), "its files went with it");
    assert!(kept.exists(), "another session's stay");
    assert!(matches!(
        store.delete_session(ended).await,
        Err(StoreError::UnknownSession(_))
    ));
    assert_eq!(stored(&store).await, vec![live]);
}

/// Retention prunes archived sessions first, then other ended ones, pinned
/// ones last, each oldest end first; a pruned session's folder goes too.
#[tokio::test]
async fn retention_prunes_archived_first_and_pinned_last() {
    let (_temp, dir) = data_dir();
    let config = StoreConfig {
        retention: Retention {
            max_ended_sessions: 3,
            ..Retention::default()
        },
        ..StoreConfig::default()
    };
    let store = open(&dir, config).await;
    let pinned = running(&store, 0).await;
    let older = running(&store, 1).await;
    let old = running(&store, 2).await;
    let archived = running(&store, 3).await;
    end(&store, pinned, 2, 10).await;
    end(&store, older, 2, 11).await;
    end(&store, old, 2, 12).await;
    let _kept = store.mark(pinned, Mark::Pin, at(13)).await.unwrap();
    // The node archives only ended sessions; the store's order does not
    // depend on it.
    let _kept = store.mark(archived, Mark::Archive, at(14)).await.unwrap();
    let folder = attach(&dir, archived);
    end(&store, archived, 2, 15).await;
    assert_eq!(
        stored(&store).await,
        sorted(vec![pinned, older, old]),
        "the archived one first, though it ended last"
    );
    assert!(!folder.exists(), "pruned with its files");

    let newer = running(&store, 4).await;
    end(&store, newer, 2, 16).await;
    assert_eq!(
        stored(&store).await,
        sorted(vec![pinned, old, newer]),
        "then the oldest end, passing over the pinned one"
    );

    let _kept = store.mark(old, Mark::Pin, at(17)).await.unwrap();
    let _kept = store.mark(newer, Mark::Pin, at(18)).await.unwrap();
    let newest = running(&store, 5).await;
    end(&store, newest, 2, 19).await;
    assert_eq!(
        stored(&store).await,
        sorted(vec![pinned, old, newer]),
        "pinned ones last: the newest unpinned goes before them"
    );
}

/// Schema 7 archived every session that had ended before it, as of its
/// end, so the board looks as it did; a session still live then is not.
/// The database is taken back to schema 6 and walks 7, 8 and 9 in order
/// (9, the workspace's tables, comes back empty).
#[tokio::test]
async fn sessions_that_ended_before_schema_7_are_archived() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let ended = running(&store, 0).await;
    end(&store, ended, 2, 5).await;
    let live = running(&store, 1).await;
    store.close().await;
    drop(store);
    let connection = rusqlite::Connection::open(dir.database_path()).unwrap();
    fixtures::undo_registry_schema(&connection);
    connection
        .execute_batch("ALTER TABLE sessions DROP COLUMN listing;")
        .unwrap();
    connection
        .execute_batch("ALTER TABLE sessions DROP COLUMN labels;")
        .unwrap();
    connection
        .execute_batch(
            "DROP TABLE terminal_profile_settings; DROP TABLE creation_claims; DROP TABLE creation_receipts;
     ALTER TABLE sessions DROP COLUMN detach_receipt;
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
             DROP TABLE attachments;",
        )
        .unwrap();
    connection.pragma_update(None, "user_version", 6).unwrap();
    drop(connection);

    let store = open(&dir, StoreConfig::default()).await;
    let workspace = store.workspace().await.unwrap();
    assert!(workspace.projects.is_empty() && workspace.worktrees.is_empty());
    let before = store.session(ended).await.unwrap().unwrap();
    assert_eq!(before.marks.archived_at, Some(at(5)));
    assert!(!before.marks.unread, "no attention point was kept before");
    assert_eq!(before.board, Some(SessionBoard::archived(at(5))));
    let interrupted = store.session(live).await.unwrap().unwrap();
    assert_eq!(interrupted.marks.archived_at, None, "it was live then");
    assert_eq!(
        interrupted.board,
        interrupted.ended_at.map(SessionBoard::exited),
        "ended at the reopen, in building"
    );
}
