//! Runtime registration of project roots (ADR 0106): a project root where work
//! starts finds or creates its `Local` registry workspace in one writer
//! savepoint; removal archives the workspaces it strands; session membership is
//! derived from each session's stored place rather than written per session.
//!
//! [Decision Log]
//! - Purpose: before this, nothing in production created a registry workspace,
//!   so labels, scripts and services had nothing to attach to.
//! - Reference: Paseo `findOrCreateWorkspaceForDirectory`
//!   (`workspace-provisioning-service.ts` 356-375) picks the oldest active,
//!   foreground workspace at the path and creates one otherwise; membership is
//!   the agent's own `workspaceId`.
//! - Choice: lookup and insert share one savepoint, so two starts at one root
//!   cannot each create a workspace; the caller passes the fresh id. No row per
//!   session or terminal: lifetime row caps would make creation refuse forever
//!   (node reviewer, coordinator ruling 2026-11-16). Worktrees are not
//!   registered here (`root()` maps `Local` to the project root).
//! - Effect: `workspace.registry` lists one workspace per opened project root
//!   with its placed, non-archived sessions as derived members.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use vessel_proto::registry::{
    RegistryRevision, WorkspaceAutomation, WorkspaceBacking, WorkspaceDescriptor,
    WorkspaceLifecycle, WorkspaceMembership, WorkspacePin, WorkspaceResource, WorkspaceVisibility,
};
use vessel_proto::{ProjectId, SessionId, WorkspaceId};

use super::{RegistryChange, RegistryRecord, decode, write};
use crate::StoreError;
use crate::codec::{session_id_from_sql, time_from_sql};

/// Archived `Local` records kept beyond this are pruned oldest first; they
/// never count toward [`super::MAX_WORKSPACES`] (ADR 0082 amendment).
pub const MAX_ARCHIVED_WORKSPACES: usize = super::MAX_WORKSPACES;

/// A project root where work starts, as the node resolved it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    /// The registered project.
    pub project: ProjectId,
    /// Its root directory.
    pub path: PathBuf,
}

/// Find or create `placement`'s workspace; a new one takes `fresh` and `at`.
#[derive(Debug)]
pub(crate) struct Register {
    pub(crate) placement: Placement,
    pub(crate) fresh: WorkspaceId,
    pub(crate) at: DateTime<Utc>,
}

pub(crate) fn register(
    connection: &Connection,
    register: &Register,
) -> Result<(WorkspaceId, RegistryChange), StoreError> {
    if let Some(found) = oldest_active(connection, register.placement.project)?
        .into_iter()
        .find(|descriptor| descriptor.path == register.placement.path)
    {
        return Ok((found.id, RegistryChange::Unchanged));
    }
    let descriptor = WorkspaceDescriptor {
        id: register.fresh,
        project: register.placement.project,
        path: register.placement.path.clone(),
        title: None,
        automation: WorkspaceAutomation::Allowed,
        pin: WorkspacePin::default(),
        backing: WorkspaceBacking::Local,
        lifecycle: WorkspaceLifecycle::Active,
        visibility: WorkspaceVisibility::Foreground,
        created_at: register.at,
        revision: RegistryRevision::ZERO,
    };
    write::put(
        connection,
        &RegistryRecord {
            descriptor,
            recovery: None,
        },
        None,
    )?;
    write::bump_revision(connection)?;
    Ok((register.fresh, RegistryChange::Changed))
}

/// The project's Active, foreground `Local` workspaces, oldest first (creation
/// time, then id), as Paseo's path discovery orders them.
fn oldest_active(
    connection: &Connection,
    project: ProjectId,
) -> Result<Vec<WorkspaceDescriptor>, StoreError> {
    let mut found: Vec<WorkspaceDescriptor> = descriptors(connection, project)?
        .into_iter()
        .filter(|descriptor| {
            descriptor.lifecycle == WorkspaceLifecycle::Active
                && descriptor.visibility == WorkspaceVisibility::Foreground
                && descriptor.backing == WorkspaceBacking::Local
        })
        .collect();
    found.sort_by_key(|descriptor| (descriptor.created_at, descriptor.id));
    Ok(found)
}

fn descriptors(
    connection: &Connection,
    project: ProjectId,
) -> Result<Vec<WorkspaceDescriptor>, StoreError> {
    let mut statement =
        connection.prepare_cached("SELECT descriptor FROM workspace_registry WHERE project=?1")?;
    let mut rows = statement.query(params![project.to_string()])?;
    let mut found = Vec::new();
    while let Some(row) = rows.next()? {
        let text: String = row.get(0)?;
        found.push(decode(&text)?);
    }
    Ok(found)
}

/// Legacy project removal archives the project's own `Local` Active
/// workspaces in its own savepoint (Paseo TRACKER 258), then prunes archived
/// `Local` records beyond [`MAX_ARCHIVED_WORKSPACES`], oldest first.
pub(crate) fn archive_stranded(
    connection: &Connection,
    project: ProjectId,
    at: DateTime<Utc>,
) -> Result<RegistryChange, StoreError> {
    let mut change = RegistryChange::Unchanged;
    for mut descriptor in descriptors(connection, project)? {
        if descriptor.backing != WorkspaceBacking::Local
            || descriptor.lifecycle != WorkspaceLifecycle::Active
        {
            continue;
        }
        let expected = descriptor.revision;
        descriptor.revision = expected
            .next()
            .ok_or(StoreError::RegistryRevisionExhausted)?;
        descriptor.lifecycle = WorkspaceLifecycle::Archived { at };
        write::put(
            connection,
            &RegistryRecord {
                descriptor,
                recovery: None,
            },
            Some(expected),
        )?;
        change = RegistryChange::Changed;
    }
    if change == RegistryChange::Changed {
        prune_archived(connection)?;
        write::bump_revision(connection)?;
    }
    Ok(change)
}

/// Keep at most [`MAX_ARCHIVED_WORKSPACES`] archived `Local` records; a live
/// create is never refused for them. Their label assignments cascade.
fn prune_archived(connection: &Connection) -> Result<(), StoreError> {
    let mut statement = connection.prepare_cached(
        "SELECT id,descriptor FROM workspace_registry \
         WHERE json_extract(descriptor,'$.lifecycle.kind')='archived' \
         AND json_extract(descriptor,'$.backing.kind')='local' \
         AND id NOT IN (SELECT id FROM workspace_registry_reservations)",
    )?;
    let mut rows = statement.query([])?;
    let mut archived: Vec<(DateTime<Utc>, WorkspaceId, String)> = Vec::new();
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let text: String = row.get(1)?;
        let descriptor: WorkspaceDescriptor = decode(&text)?;
        if let WorkspaceLifecycle::Archived { at } = descriptor.lifecycle {
            archived.push((at, descriptor.id, id));
        }
    }
    let excess = archived.len().saturating_sub(MAX_ARCHIVED_WORKSPACES);
    archived.sort_by_key(|(at, id, _)| (*at, *id));
    for (_, _, id) in archived.into_iter().take(excess) {
        let _deleted = connection.execute("DELETE FROM workspace_registry WHERE id=?1", [id])?;
    }
    Ok(())
}

/// Each placed, non-archived session at a project root belongs to that
/// project's oldest Active `Local` workspace (Paseo's `agent.workspaceId`),
/// unless an explicit membership row already names it.
pub(crate) fn derived_memberships(
    connection: &Connection,
    workspaces: &[WorkspaceDescriptor],
    explicit: &[WorkspaceMembership],
) -> Result<Vec<WorkspaceMembership>, StoreError> {
    let mut owners: Vec<(ProjectId, WorkspaceId, DateTime<Utc>)> = Vec::new();
    for descriptor in workspaces {
        let live = descriptor.lifecycle == WorkspaceLifecycle::Active
            && descriptor.visibility == WorkspaceVisibility::Foreground
            && descriptor.backing == WorkspaceBacking::Local;
        if !live {
            continue;
        }
        match owners.iter_mut().find(|(project, ..)| *project == descriptor.project) {
            Some(owner) if (descriptor.created_at, descriptor.id) < (owner.2, owner.1) => {
                owner.1 = descriptor.id;
                owner.2 = descriptor.created_at;
            }
            Some(_) => {}
            None => owners.push((descriptor.project, descriptor.id, descriptor.created_at)),
        }
    }
    let mut derived = Vec::new();
    for (project, workspace, _) in owners {
        for session in placed_sessions(connection, project)? {
            let resource = WorkspaceResource::Session { session };
            if !explicit.iter().any(|member| member.resource == resource) {
                derived.push(WorkspaceMembership {
                    workspace,
                    resource,
                });
            }
        }
    }
    Ok(derived)
}

fn placed_sessions(connection: &Connection, project: ProjectId) -> Result<Vec<SessionId>, StoreError> {
    let mut statement = connection.prepare_cached(
        "SELECT id FROM sessions WHERE archived_at IS NULL \
         AND json_extract(spec,'$.place.kind')='project' \
         AND json_extract(spec,'$.place.project')=?1 ORDER BY created_at,id",
    )?;
    let mut rows = statement.query(params![project.to_string()])?;
    let mut sessions = Vec::new();
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        sessions.push(session_id_from_sql(&id)?);
    }
    Ok(sessions)
}

/// Bootstrap (node start, one shot per project): projects that have placed,
/// non-archived sessions and no registry record in any lifecycle, each with
/// its earliest such session's creation time.
pub(crate) fn unregistered_placed(
    connection: &Connection,
) -> Result<Vec<(ProjectId, DateTime<Utc>)>, StoreError> {
    let mut statement = connection.prepare_cached(
        "SELECT json_extract(spec,'$.place.project'), MIN(created_at) FROM sessions \
         WHERE archived_at IS NULL AND json_extract(spec,'$.place.kind')='project' \
         AND json_extract(spec,'$.place.project') NOT IN (SELECT project FROM workspace_registry) \
         GROUP BY json_extract(spec,'$.place.project') \
         ORDER BY json_extract(spec,'$.place.project')",
    )?;
    let mut rows = statement.query([])?;
    let mut found = Vec::new();
    while let Some(row) = rows.next()? {
        let project: String = row.get(0)?;
        let earliest: i64 = row.get(1)?;
        let project: ProjectId =
            decode(&serde_json::to_string(&project).map_err(StoreError::Encode)?)?;
        found.push((project, time_from_sql(earliest)));
    }
    Ok(found)
}
