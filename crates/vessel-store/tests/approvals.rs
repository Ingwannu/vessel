//! Approved command sets across the schemas (protocol 10's schema 9,
//! later schemas): an approval kept before the store held what it covered
//! survives the walk to the current schema in order, with no set; one
//! given after keeps its set across a restart.

use build::{digest, project};
use fixtures::{at, data_dir, open};
use vessel_proto::workspace::{ApprovedCommands, CommandSet};
use vessel_store::{ApprovalChange, StoreConfig};

#[cfg(test)]
#[path = "support/fixtures.rs"]
#[expect(
    dead_code,
    reason = "builders shared with the log's tests; this file uses some"
)]
mod fixtures;

/// Undoes schemas 16 to 10: the inbox and push subscriptions, the usage
/// ledger, side plans, search,
/// schedules, saved prompts, hooks, the set an approval covered, and pull
/// request feedback.
const UNDO_SCHEMAS_10_TO_16: &str = "ALTER TABLE sessions DROP COLUMN detach_receipt;
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
     ALTER TABLE approvals DROP COLUMN commands;
     DROP TABLE forge_feedback_sent;
     DROP TABLE forge_feedback_on;";

/// Builders, in a `cfg(test)` module for the lints' test allowances.
#[cfg(test)]
mod build {
    use vessel_proto::ProjectId;
    use vessel_proto::workspace::{CommandsDigest, ProjectInfo, ProjectState};

    use super::fixtures::at;

    pub(crate) fn project() -> ProjectInfo {
        ProjectInfo {
            id: ProjectId::random(),
            name: "app".parse().unwrap(),
            root: std::path::PathBuf::from("/work/app"),
            git: true,
            default_branch: None,
            state: ProjectState::Ready,
            created_at: at(0),
            revision: 0,
        }
    }

    pub(crate) fn digest(digit: char) -> CommandsDigest {
        digit.to_string().repeat(64).parse().unwrap()
    }
}

#[tokio::test]
async fn an_old_approval_survives_current_migrations_and_new_ones_keep_their_set() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let app = project();
    store.put_project(app.clone()).await.unwrap();
    store.close().await;
    drop(store);

    // A database as protocol 10 left it, with an approval in it.
    let connection = rusqlite::Connection::open(dir.database_path()).unwrap();
    connection
        .execute_batch("ALTER TABLE sessions DROP COLUMN updated_at;")
        .unwrap();
    connection
        .execute_batch("ALTER TABLE sessions DROP COLUMN listing;")
        .unwrap();
    connection
        .execute_batch("ALTER TABLE sessions DROP COLUMN labels;")
        .unwrap();
    fixtures::undo_registry_schema(&connection);
    connection.execute_batch(UNDO_SCHEMAS_10_TO_16).unwrap();
    let _row = connection
        .execute(
            "INSERT INTO approvals (project, digest, approved_at) VALUES (?1, ?2, 1)",
            [app.id.to_string(), "a".repeat(64)],
        )
        .unwrap();
    connection.pragma_update(None, "user_version", 9).unwrap();
    drop(connection);

    let store = open(&dir, StoreConfig::default()).await;
    let walked = store.workspace().await.unwrap();
    assert_eq!(walked.approvals, [(app.id, digest('a'))]);
    assert!(walked.approved_sets.is_empty(), "it covered nothing known");
    // Feedback's tables came back with schema 10, the schedules' with 12,
    // the search index with 13.
    assert!(
        store
            .feedback(vessel_proto::SessionId::random())
            .await
            .is_ok()
    );
    let automation = store.automation().await.unwrap();
    assert!(automation.schedules.is_empty());
    assert!(store.saved_prompts(None).await.unwrap().is_empty());
    store.indexed().await;
    assert!(
        store
            .search(vessel_proto::search::SearchMessages {
                query: "anything".parse().unwrap(),
                session: None,
                before: None,
                limit: None,
            })
            .await
            .unwrap()
            .hits
            .is_empty()
    );

    let set = CommandSet {
        setup: vec!["npm ci".parse().unwrap()],
        ..CommandSet::default()
    };
    store
        .set_approval(
            app.id,
            digest('b'),
            ApprovalChange::Approved {
                at: at(5),
                commands: Some(Box::new(set.clone())),
            },
        )
        .await
        .unwrap();
    store.close().await;
    drop(store);

    let store = open(&dir, StoreConfig::default()).await;
    let read = store.workspace().await.unwrap();
    assert_eq!(read.approvals.len(), 2);
    assert_eq!(
        read.approved_sets,
        [(
            app.id,
            ApprovedCommands {
                commands: set,
                digest: digest('b'),
                approved_at: at(5),
            }
        )]
    );
}
