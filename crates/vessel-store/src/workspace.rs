//! Projects, worktrees and approved commands (protocol 10, schema 9, ADR
//! 0027): the node's own record of the folders it works in.
//!
//! [Decision Log]
//! - Purpose: projects, the worktrees the node made and the command sets a
//!   person approved survive a restart; an approval in particular must not
//!   be asked again because the node stopped.
//! - Choice: a project and a worktree are each one row holding its
//!   `vessel-proto` type as JSON, as a session's spec and status are: the
//!   node keeps them in memory and writes a row whole when one changes, so
//!   a field added to the wire type needs no migration. What is live (a
//!   worktree's branch now, its sessions, a project's revision) is filled
//!   by the node when it answers, never stored. An approval is a row of its
//!   own per project and digest, deleted with its project.
//! - Rows that no longer decode are skipped with a warning when loaded: a
//!   bad row loses that one record, not the node's start.

use rusqlite::{Connection, params};
use vessel_proto::workspace::{CommandsDigest, ProjectInfo, WorktreeInfo};
use vessel_proto::{ProjectId, WorktreeId};

use crate::codec::{encode, time_to_sql};
use crate::error::StoreError;

/// What the store holds about projects, read once when the node starts.
#[derive(Debug, Clone, Default)]
pub struct StoredWorkspace {
    /// Every project, oldest first.
    pub projects: Vec<ProjectInfo>,
    /// Every worktree, oldest first.
    pub worktrees: Vec<WorktreeInfo>,
    /// Every approved command set.
    pub approvals: Vec<(ProjectId, CommandsDigest)>,
}

/// A change to that record.
#[derive(Debug)]
pub(crate) enum WorkspaceWrite {
    /// Insert or replace a project.
    Project(Box<ProjectInfo>),
    /// Delete a project and its approvals.
    ForgetProject(ProjectId),
    /// Insert or replace a worktree.
    Worktree(Box<WorktreeInfo>),
    /// Delete a worktree.
    ForgetWorktree(WorktreeId),
    /// Approve a set, or withdraw an approval.
    Approval {
        project: ProjectId,
        digest: CommandsDigest,
        approved: bool,
    },
}

/// Apply one change. Writer side.
pub(crate) fn apply(connection: &Connection, write: &WorkspaceWrite) -> Result<(), StoreError> {
    let changed = match write {
        WorkspaceWrite::Project(project) => connection
            .prepare_cached(
                "INSERT INTO projects (id, created_at, info) VALUES (?1, ?2, ?3)
                 ON CONFLICT (id) DO UPDATE SET info = excluded.info",
            )?
            .execute(params![
                project.id.to_string(),
                time_to_sql(project.created_at)?,
                encode(project)?
            ])?,
        WorkspaceWrite::ForgetProject(id) => {
            let _approvals = connection
                .prepare_cached("DELETE FROM approvals WHERE project = ?1")?
                .execute([id.to_string()])?;
            connection
                .prepare_cached("DELETE FROM projects WHERE id = ?1")?
                .execute([id.to_string()])?
        }
        WorkspaceWrite::Worktree(worktree) => connection
            .prepare_cached(
                "INSERT INTO worktrees (id, project, created_at, info) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (id) DO UPDATE SET info = excluded.info",
            )?
            .execute(params![
                worktree.id.to_string(),
                worktree.project.to_string(),
                time_to_sql(worktree.created_at)?,
                encode(worktree)?
            ])?,
        WorkspaceWrite::ForgetWorktree(id) => connection
            .prepare_cached("DELETE FROM worktrees WHERE id = ?1")?
            .execute([id.to_string()])?,
        WorkspaceWrite::Approval {
            project,
            digest,
            approved: true,
        } => connection
            .prepare_cached(
                "INSERT OR IGNORE INTO approvals (project, digest, approved_at)
                 VALUES (?1, ?2, ?3)",
            )?
            .execute(params![
                project.to_string(),
                digest.as_str(),
                time_to_sql(chrono::Utc::now())?
            ])?,
        WorkspaceWrite::Approval {
            project,
            digest,
            approved: false,
        } => connection
            .prepare_cached("DELETE FROM approvals WHERE project = ?1 AND digest = ?2")?
            .execute(params![project.to_string(), digest.as_str()])?,
    };
    tracing::trace!(rows = changed, "workspace record written");
    Ok(())
}

/// Everything, oldest first. Reader side.
pub(crate) fn load(connection: &Connection) -> Result<StoredWorkspace, StoreError> {
    let mut stored = StoredWorkspace::default();
    let mut statement =
        connection.prepare_cached("SELECT info FROM projects ORDER BY created_at, id")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let text: String = row.get(0)?;
        match serde_json::from_str(&text) {
            Ok(project) => stored.projects.push(project),
            Err(error) => tracing::warn!(%error, "a stored project does not decode; skipped"),
        }
    }
    let mut statement =
        connection.prepare_cached("SELECT info FROM worktrees ORDER BY created_at, id")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let text: String = row.get(0)?;
        match serde_json::from_str(&text) {
            Ok(worktree) => stored.worktrees.push(worktree),
            Err(error) => tracing::warn!(%error, "a stored worktree does not decode; skipped"),
        }
    }
    let mut statement = connection.prepare_cached("SELECT project, digest FROM approvals")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let project: String = row.get(0)?;
        let digest: String = row.get(1)?;
        if let (Ok(project), Ok(digest)) = (project.parse(), digest.parse()) {
            stored.approvals.push((project, digest));
        } else {
            tracing::warn!("a stored approval does not decode; skipped");
        }
    }
    Ok(stored)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;
    use vessel_proto::workspace::{ProjectState, SetupState};
    use vessel_proto::{Creator, SessionId};

    use super::*;

    fn project() -> ProjectInfo {
        ProjectInfo {
            id: ProjectId::random(),
            name: "app".parse().unwrap(),
            root: PathBuf::from("/work/app"),
            git: true,
            default_branch: None,
            state: ProjectState::Ready,
            created_at: Utc::now(),
            revision: 0,
        }
    }

    fn worktree(project: ProjectId) -> WorktreeInfo {
        WorktreeInfo {
            id: WorktreeId::random(),
            project,
            path: PathBuf::from("/data/worktrees/x"),
            branch: None,
            created_branch: Some("vessel/abcd1234".parse().unwrap()),
            base: None,
            created_at: Utc::now(),
            created_by: Creator::Owner,
            created_for: Some(SessionId::random()),
            sessions: Vec::new(),
            setup: SetupState::None,
            kept: None,
        }
    }

    #[test]
    fn projects_worktrees_and_approvals_round_trip() {
        let mut connection = Connection::open_in_memory().unwrap();
        crate::schema::migrate(&mut connection).unwrap();
        let app = project();
        let tree = worktree(app.id);
        let digest: CommandsDigest = "a".repeat(64).parse().unwrap();
        for write in [
            WorkspaceWrite::Project(Box::new(app.clone())),
            WorkspaceWrite::Worktree(Box::new(tree.clone())),
            WorkspaceWrite::Approval {
                project: app.id,
                digest: digest.clone(),
                approved: true,
            },
            // Approving twice keeps one row.
            WorkspaceWrite::Approval {
                project: app.id,
                digest: digest.clone(),
                approved: true,
            },
        ] {
            apply(&connection, &write).unwrap();
        }
        let renamed = ProjectInfo {
            name: "renamed".parse().unwrap(),
            ..app.clone()
        };
        apply(
            &connection,
            &WorkspaceWrite::Project(Box::new(renamed.clone())),
        )
        .unwrap();
        let stored = load(&connection).unwrap();
        assert_eq!(stored.projects, [renamed]);
        assert_eq!(stored.worktrees, std::slice::from_ref(&tree));
        assert_eq!(stored.approvals, [(app.id, digest.clone())]);

        apply(
            &connection,
            &WorkspaceWrite::Approval {
                project: app.id,
                digest,
                approved: false,
            },
        )
        .unwrap();
        apply(&connection, &WorkspaceWrite::ForgetWorktree(tree.id)).unwrap();
        apply(&connection, &WorkspaceWrite::ForgetProject(app.id)).unwrap();
        let empty = load(&connection).unwrap();
        assert!(
            empty.projects.is_empty() && empty.worktrees.is_empty() && empty.approvals.is_empty()
        );
    }
}
