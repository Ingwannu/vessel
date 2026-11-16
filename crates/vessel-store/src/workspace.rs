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
//!   own per project and digest, deleted with its project; from schema 11
//!   it holds the set it approved too, and a second approval of a digest
//!   moves its time (so the newest is the last approved).
//! - Rows that no longer decode are skipped with a warning when loaded: a
//!   bad row loses that one record, not the node's start.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use vessel_proto::workspace::{
    ApprovedCommands, CommandSet, CommandsDigest, ProjectInfo, WorktreeInfo,
};
use vessel_proto::{ProjectId, WorktreeId};

use crate::codec::{encode, time_from_sql, time_to_sql};
use crate::error::StoreError;

/// The most approved sets read back per project, newest first: the node
/// shows one, and keeps a few for when the newest is withdrawn.
pub const MAX_APPROVED_SETS: usize = 4;

/// The largest stored set read back, as JSON: a set comes from a file of
/// at most 64 KiB, which its JSON exceeds only by escaping. A larger one
/// was not written by this node and is skipped.
const MAX_SET_JSON: usize = 256 * 1024;

/// What the store holds about projects, read once when the node starts.
#[derive(Debug, Clone, Default)]
pub struct StoredWorkspace {
    /// Every project, oldest first.
    pub projects: Vec<ProjectInfo>,
    /// Every worktree, oldest first.
    pub worktrees: Vec<WorktreeInfo>,
    /// Every approved command set.
    pub approvals: Vec<(ProjectId, CommandsDigest)>,
    /// The sets approved since schema 11, newest first, at most
    /// [`MAX_APPROVED_SETS`] per project.
    pub approved_sets: Vec<(ProjectId, ApprovedCommands)>,
}

/// What a person did with a set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalChange {
    /// Approved it at `at`: the set, when the node had described it.
    Approved {
        /// When.
        at: DateTime<Utc>,
        /// What it covers.
        commands: Option<Box<CommandSet>>,
    },
    /// Withdrew the approval.
    Withdrawn,
}

/// A change to that record.
#[derive(Debug)]
pub(crate) enum WorkspaceWrite {
    /// Insert or replace a project.
    Project(Box<ProjectInfo>),
    /// Delete a project and its approvals, archiving the `Local` Active
    /// registry workspaces that stood for it (ADR 0106) at `at`.
    ForgetProject {
        project: ProjectId,
        at: DateTime<Utc>,
    },
    /// Insert or replace a worktree.
    Worktree(Box<WorktreeInfo>),
    /// Delete a worktree.
    ForgetWorktree(WorktreeId),
    /// Approve a set, or withdraw an approval.
    Approval {
        project: ProjectId,
        digest: CommandsDigest,
        change: ApprovalChange,
    },
}

/// Apply one change. Writer side. Whether the registry changed, so its
/// observers wake once the batch commits.
pub(crate) fn apply(
    connection: &Connection,
    write: &WorkspaceWrite,
) -> Result<crate::registry::RegistryChange, StoreError> {
    let mut registry = crate::registry::RegistryChange::Unchanged;
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
        WorkspaceWrite::ForgetProject { project, at } => {
            registry = crate::registry::archive_stranded(connection, *project, *at)?;
            let _approvals = connection
                .prepare_cached("DELETE FROM approvals WHERE project = ?1")?
                .execute([project.to_string()])?;
            connection
                .prepare_cached("DELETE FROM projects WHERE id = ?1")?
                .execute([project.to_string()])?
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
            change: ApprovalChange::Approved { at, commands },
        } => approve(connection, *project, digest, *at, commands.as_deref())?,
        WorkspaceWrite::Approval {
            project,
            digest,
            change: ApprovalChange::Withdrawn,
        } => connection
            .prepare_cached("DELETE FROM approvals WHERE project = ?1 AND digest = ?2")?
            .execute(params![project.to_string(), digest.as_str()])?,
    };
    tracing::trace!(rows = changed, "workspace record written");
    Ok(registry)
}

/// Approve `digest` for `project` at `at`, with the set it covers when
/// known (approved again: the newest time, and the set once known); then
/// keep the sets of the newest [`MAX_APPROVED_SETS`] approvals of the
/// project only, so the rows a person's approvals leave stay small (an
/// older approval keeps its digest, and so its effect).
fn approve(
    connection: &Connection,
    project: ProjectId,
    digest: &CommandsDigest,
    at: DateTime<Utc>,
    commands: Option<&CommandSet>,
) -> Result<usize, StoreError> {
    let changed = connection
        .prepare_cached(
            "INSERT INTO approvals (project, digest, approved_at, commands)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (project, digest) DO UPDATE SET
                 approved_at = excluded.approved_at,
                 commands = coalesce(excluded.commands, approvals.commands)",
        )?
        .execute(params![
            project.to_string(),
            digest.as_str(),
            time_to_sql(at)?,
            commands.map(encode).transpose()?
        ])?;
    let _older = connection
        .prepare_cached(
            "UPDATE approvals SET commands = NULL
             WHERE project = ?1 AND commands IS NOT NULL AND digest NOT IN (
                 SELECT digest FROM approvals
                 WHERE project = ?1 AND commands IS NOT NULL
                 ORDER BY approved_at DESC, digest LIMIT ?2
             )",
        )?
        .execute(params![
            project.to_string(),
            i64::try_from(MAX_APPROVED_SETS).unwrap_or(i64::MAX)
        ])?;
    Ok(changed)
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
    stored.approved_sets = approved_sets(connection)?;
    Ok(stored)
}

/// The sets approved since schema 11, newest first, [`MAX_APPROVED_SETS`]
/// per project at most. A row that does not decode, or is larger than a
/// set this node writes, is skipped.
fn approved_sets(
    connection: &Connection,
) -> Result<Vec<(ProjectId, ApprovedCommands)>, StoreError> {
    let mut statement = connection.prepare_cached(
        "SELECT project, digest, approved_at, commands FROM approvals
         WHERE commands IS NOT NULL ORDER BY approved_at DESC, digest",
    )?;
    let mut rows = statement.query([])?;
    let mut sets: Vec<(ProjectId, ApprovedCommands)> = Vec::new();
    while let Some(row) = rows.next()? {
        let (project, digest, at, text): (String, String, i64, String) =
            (row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?);
        let (Ok(project), Ok(digest)) = (project.parse::<ProjectId>(), digest.parse()) else {
            tracing::warn!("a stored approved set does not decode; skipped");
            continue;
        };
        let kept = sets.iter().filter(|(owner, _)| *owner == project).count();
        if kept >= MAX_APPROVED_SETS {
            continue;
        }
        if text.len() > MAX_SET_JSON {
            tracing::warn!(
                bytes = text.len(),
                "a stored approved set is too large; skipped"
            );
            continue;
        }
        match serde_json::from_str::<CommandSet>(&text) {
            Ok(commands) => sets.push((
                project,
                ApprovedCommands {
                    commands,
                    digest,
                    approved_at: time_from_sql(at),
                },
            )),
            Err(error) => tracing::warn!(%error, "a stored approved set does not decode; skipped"),
        }
    }
    Ok(sets)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;
    use vessel_proto::workspace::{ProjectState, SetupState};
    use vessel_proto::{Creator, SessionId};

    use super::*;

    fn at(second: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000_i64.saturating_add(second), 0).unwrap()
    }

    fn set(command: &str) -> CommandSet {
        CommandSet {
            setup: vec![command.parse().unwrap()],
            ..CommandSet::default()
        }
    }

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
            // Before schema 11's set was known.
            WorkspaceWrite::Approval {
                project: app.id,
                digest: digest.clone(),
                change: ApprovalChange::Approved {
                    at: at(1),
                    commands: None,
                },
            },
            // Approving twice keeps one row, the newest time, and the set
            // once known.
            WorkspaceWrite::Approval {
                project: app.id,
                digest: digest.clone(),
                change: ApprovalChange::Approved {
                    at: at(2),
                    commands: Some(Box::new(set("npm ci"))),
                },
            },
            WorkspaceWrite::Approval {
                project: app.id,
                digest: digest.clone(),
                change: ApprovalChange::Approved {
                    at: at(3),
                    commands: None,
                },
            },
        ] {
            let _change = apply(&connection, &write).unwrap();
        }
        let renamed = ProjectInfo {
            name: "renamed".parse().unwrap(),
            ..app.clone()
        };
        let _change = apply(
            &connection,
            &WorkspaceWrite::Project(Box::new(renamed.clone())),
        )
        .unwrap();
        let stored = load(&connection).unwrap();
        assert_eq!(stored.projects, [renamed]);
        assert_eq!(stored.worktrees, std::slice::from_ref(&tree));
        assert_eq!(stored.approvals, [(app.id, digest.clone())]);
        assert_eq!(
            stored.approved_sets,
            [(
                app.id,
                ApprovedCommands {
                    commands: set("npm ci"),
                    digest: digest.clone(),
                    approved_at: at(3),
                }
            )]
        );

        let _change = apply(
            &connection,
            &WorkspaceWrite::Approval {
                project: app.id,
                digest,
                change: ApprovalChange::Withdrawn,
            },
        )
        .unwrap();
        let _change = apply(&connection, &WorkspaceWrite::ForgetWorktree(tree.id)).unwrap();
        let _change = apply(
            &connection,
            &WorkspaceWrite::ForgetProject {
                project: app.id,
                at: Utc::now(),
            },
        )
        .unwrap();
        let empty = load(&connection).unwrap();
        assert!(
            empty.projects.is_empty() && empty.worktrees.is_empty() && empty.approvals.is_empty()
        );
        assert!(empty.approved_sets.is_empty());
    }

    /// Only the newest approvals of a project keep their sets (every
    /// approval keeps its digest); they come back newest first, and a row
    /// that is not one this node wrote is skipped, not fatal.
    #[test]
    fn approved_sets_come_back_newest_first_and_bounded() {
        let mut connection = Connection::open_in_memory().unwrap();
        crate::schema::migrate(&mut connection).unwrap();
        let app = project();
        let _change = apply(&connection, &WorkspaceWrite::Project(Box::new(app.clone()))).unwrap();
        for second in 0..6_i64 {
            let digest: CommandsDigest = format!("{second:064}").parse().unwrap();
            let _change = apply(
                &connection,
                &WorkspaceWrite::Approval {
                    project: app.id,
                    digest,
                    change: ApprovalChange::Approved {
                        at: at(second),
                        commands: Some(Box::new(set(&format!("echo {second}")))),
                    },
                },
            )
            .unwrap();
        }
        let kept: i64 = connection
            .query_row(
                "SELECT count(*) FROM approvals WHERE commands IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(kept, i64::try_from(MAX_APPROVED_SETS).unwrap());
        assert_eq!(load(&connection).unwrap().approvals.len(), 6);
        let _broken = connection
            .execute(
                r#"UPDATE approvals SET commands = '{"setup": 7}' WHERE digest = ?1"#,
                [format!("{:064}", 5)],
            )
            .unwrap();
        let sets = load(&connection).unwrap().approved_sets;
        let newest: Vec<String> = sets
            .iter()
            .map(|(_, set)| set.commands.setup[0].as_str().to_owned())
            .collect();
        assert_eq!(newest, ["echo 4", "echo 3", "echo 2"]);
    }
}
