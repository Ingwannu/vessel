//! Approved command sets across the schemas (protocol 10's schema 9,
//! protocol 11's 10 and 11): an approval kept before the store held what
//! it covered survives the walk from 9 to 11 in order, with no set; one
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

/// Undoes schemas 11 and 10: the set an approval covered, and pull
/// request feedback's tables.
const UNDO_SCHEMAS_10_AND_11: &str = "ALTER TABLE approvals DROP COLUMN commands;
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
async fn an_approval_from_schema_9_walks_to_11_and_new_ones_keep_their_set() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let app = project();
    store.put_project(app.clone()).await.unwrap();
    store.close().await;
    drop(store);

    // A database as protocol 10 left it, with an approval in it.
    let connection = rusqlite::Connection::open(dir.database_path()).unwrap();
    connection.execute_batch(UNDO_SCHEMAS_10_AND_11).unwrap();
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
    // Feedback's tables came back with schema 10.
    assert!(
        store
            .feedback(vessel_proto::SessionId::random())
            .await
            .is_ok()
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
