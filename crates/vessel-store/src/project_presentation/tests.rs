use super::*;
use vessel_proto::workspace::{CloneStage, ProjectState};

fn project() -> ProjectInfo {
    ProjectInfo {
        id: ProjectId::random(),
        name: "app".parse().unwrap(),
        root: "/node/app".into(),
        git: true,
        default_branch: None,
        state: ProjectState::Ready,
        created_at: chrono::DateTime::UNIX_EPOCH,
        revision: 0,
    }
}

fn database(info: &ProjectInfo) -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch("PRAGMA foreign_keys=ON;
        CREATE TABLE projects(id TEXT NOT NULL PRIMARY KEY, created_at INTEGER NOT NULL, info TEXT NOT NULL) STRICT;").unwrap();
    connection.execute_batch(SQL).unwrap();
    let _changed = crate::workspace::apply(
        &connection,
        &crate::workspace::WorkspaceWrite::Project(Box::new(info.clone())),
    )
    .unwrap();
    connection
}

fn committed_rename(connection: &mut Connection, request: &RenameProject) -> ProjectMetadata {
    let transaction = connection.transaction().unwrap();
    let metadata = rename(&transaction, request).unwrap();
    transaction.commit().unwrap();
    metadata
}

#[test]
fn legacy_override_import_and_reset_preserve_identity_path_and_other_state() {
    let mut original = project();
    original.name = "Legacy custom name".parse().unwrap();
    let mut connection = database(&original);
    let imported = read(&connection, original.id).unwrap();
    assert_eq!(imported.derived_name.as_str(), "app");
    assert_eq!(imported.custom_name, Some(original.name.clone()));
    let reset = committed_rename(
        &mut connection,
        &RenameProject {
            project: original.id,
            custom_name: None,
        },
    );
    assert_eq!(reset.custom_name, None);
    let stored = project_info(&connection, original.id).unwrap();
    assert_eq!(stored.name.as_str(), "app");
    assert_eq!(stored.id, original.id);
    assert_eq!(stored.root, original.root);
    assert_eq!(stored.state, original.state);
    assert_eq!(read(&connection, original.id).unwrap(), reset);
}

#[test]
fn rename_keeps_latest_clone_state_and_late_progress_cannot_undo_the_override_when_projected() {
    let mut original = project();
    original.state = ProjectState::Cloning {
        stage: CloneStage::Connecting,
        percent: None,
    };
    let mut connection = database(&original);
    let renamed = committed_rename(
        &mut connection,
        &RenameProject {
            project: original.id,
            custom_name: Some("Current label".parse().unwrap()),
        },
    );
    assert_eq!(
        project_info(&connection, original.id).unwrap().state,
        original.state
    );
    let mut late = original.clone();
    late.state = ProjectState::Ready;
    late.revision = 2;
    let projected = overlay(&connection, &late).unwrap();
    let _changed = crate::workspace::apply(
        &connection,
        &crate::workspace::WorkspaceWrite::Project(Box::new(projected)),
    )
    .unwrap();
    let stored = project_info(&connection, original.id).unwrap();
    assert_eq!(stored.name, renamed.custom_name.unwrap());
    assert_eq!(stored.state, ProjectState::Ready);
    let _reset = committed_rename(
        &mut connection,
        &RenameProject {
            project: original.id,
            custom_name: None,
        },
    );
    assert_eq!(
        project_info(&connection, original.id).unwrap().state,
        ProjectState::Ready
    );
}

#[test]
fn failed_writer_savepoint_leaves_neither_half_of_a_presentation_change() {
    let original = project();
    let mut connection = database(&original);
    connection.execute_batch("CREATE TRIGGER reject_name BEFORE UPDATE ON projects BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    {
        let transaction = connection.transaction().unwrap();
        assert!(matches!(
            rename(
                &transaction,
                &RenameProject {
                    project: original.id,
                    custom_name: Some("Rejected label".parse().unwrap())
                }
            ),
            Err(PresentationError::Sql(_))
        ));
        transaction.rollback().unwrap();
    }
    assert_eq!(project_info(&connection, original.id).unwrap(), original);
    assert!(recorded(&connection, original.id).unwrap().is_none());
}

#[test]
fn missing_corrupt_identity_and_exhaustion_are_refused_without_a_new_override() {
    let mut original = project();
    original.revision = u64::MAX;
    let connection = database(&original);
    let missing = ProjectId::random();
    assert!(
        matches!(read(&connection, missing), Err(PresentationError::Missing(id)) if id == missing)
    );
    assert!(
        matches!(rename(&connection, &RenameProject { project: original.id, custom_name: Some("Cannot change".parse().unwrap()) }), Err(PresentationError::Exhausted(id)) if id == original.id)
    );
    assert!(recorded(&connection, original.id).unwrap().is_none());
    let mut wrong = imported(&original).unwrap();
    wrong.project = ProjectId::random();
    let _changed = connection
        .execute(
            "INSERT INTO project_presentation(project,presentation) VALUES (?1,?2)",
            params![
                original.id.to_string(),
                serde_json::to_string(&wrong).unwrap()
            ],
        )
        .unwrap();
    assert!(
        matches!(read(&connection, original.id), Err(PresentationError::Identity(id)) if id == original.id)
    );
}
