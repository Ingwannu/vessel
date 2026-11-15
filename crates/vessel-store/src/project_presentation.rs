//! Staged project-name SQL helpers; no migration, writer or RPC is wired yet.
//! The real writer must call rename inside its existing isolated savepoint and
//! project generic progress snapshots through overlay before persisting them.

use rusqlite::{Connection, OptionalExtension as _, params};
use vessel_proto::{
    ProjectId,
    registry::{ProjectMetadata, RenameProject},
    workspace::{InvalidText, ProjectInfo, ProjectName},
};

/// Appended only by the real C1 wiring migration, never applied at runtime here.
pub(crate) const SQL: &str = "
CREATE TABLE project_presentation (
    project TEXT NOT NULL PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    presentation TEXT NOT NULL
) STRICT, WITHOUT ROWID;
";

const FALLBACK_NAME: &str = "project";

#[derive(Debug, thiserror::Error)]
pub(crate) enum PresentationError {
    #[error("no registered project {0}")]
    Missing(ProjectId),
    #[error("project presentation identity does not match {0}")]
    Identity(ProjectId),
    #[error("project revision is exhausted for {0}")]
    Exhausted(ProjectId),
    #[error("project presentation database operation failed")]
    Sql(#[from] rusqlite::Error),
    #[error("stored project presentation is invalid")]
    Decode(#[source] serde_json::Error),
    #[error("project presentation could not be encoded")]
    Encode(#[source] serde_json::Error),
    #[error("the project presentation label is invalid")]
    Name(#[source] InvalidText),
}

/// Read presentation, importing a legacy non-derived label as an override.
/// No row is created merely by reading; the first actual rename persists it.
pub(crate) fn read(
    connection: &Connection,
    project: ProjectId,
) -> Result<ProjectMetadata, PresentationError> {
    let info = project_info(connection, project)?;
    match recorded(connection, project)? {
        Some(metadata) => Ok(metadata),
        None => imported(&info),
    }
}

/// Caller owns a savepoint: presentation and its legacy JSON projection commit together.
/// Read the current SQL row, so a captured node snapshot cannot regress clone progress.
pub(crate) fn rename(
    connection: &Connection,
    request: &RenameProject,
) -> Result<ProjectMetadata, PresentationError> {
    let mut info = project_info(connection, request.project)?;
    let mut metadata =
        recorded(connection, request.project)?.map_or_else(|| imported(&info), Ok)?;
    metadata.custom_name = request
        .normalized_custom_name()
        .map_err(PresentationError::Name)?;
    info.name = resolved(&metadata).clone();
    info.revision = info
        .revision
        .checked_add(1)
        .ok_or(PresentationError::Exhausted(info.id))?;
    let presentation = serde_json::to_string(&metadata).map_err(PresentationError::Encode)?;
    let json = serde_json::to_string(&info).map_err(PresentationError::Encode)?;
    let _changed = connection.execute(
        "INSERT INTO project_presentation(project,presentation) VALUES (?1,?2)
         ON CONFLICT(project) DO UPDATE SET presentation=excluded.presentation",
        params![request.project.to_string(), presentation],
    )?;
    let _changed = connection.execute(
        "UPDATE projects SET info=?2 WHERE id=?1",
        params![request.project.to_string(), json],
    )?;
    Ok(metadata)
}

/// The real generic project writer must apply this to an incoming clone/progress snapshot.
/// It preserves that snapshot's live state and reapplies only authoritative presentation.
pub(crate) fn overlay(
    connection: &Connection,
    incoming: &ProjectInfo,
) -> Result<ProjectInfo, PresentationError> {
    let mut info = incoming.clone();
    if let Some(metadata) = recorded(connection, incoming.id)? {
        info.name = resolved(&metadata).clone();
        let current = project_info(connection, incoming.id)?;
        info.revision = info.revision.max(current.revision);
    }
    Ok(info)
}

fn resolved(metadata: &ProjectMetadata) -> &ProjectName {
    metadata
        .custom_name
        .as_ref()
        .unwrap_or(&metadata.derived_name)
}

fn imported(info: &ProjectInfo) -> Result<ProjectMetadata, PresentationError> {
    let derived_name = match info
        .root
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.parse().ok())
    {
        Some(name) => name,
        None => FALLBACK_NAME.parse().map_err(PresentationError::Name)?,
    };
    let custom_name = (info.name != derived_name).then(|| info.name.clone());
    Ok(ProjectMetadata {
        project: info.id,
        derived_name,
        custom_name,
    })
}

fn project_info(
    connection: &Connection,
    project: ProjectId,
) -> Result<ProjectInfo, PresentationError> {
    let text: Option<String> = connection
        .query_row(
            "SELECT info FROM projects WHERE id=?1",
            [project.to_string()],
            |row| row.get(0),
        )
        .optional()?;
    let info: ProjectInfo = serde_json::from_str(&text.ok_or(PresentationError::Missing(project))?)
        .map_err(PresentationError::Decode)?;
    if info.id != project {
        return Err(PresentationError::Identity(project));
    }
    Ok(info)
}

fn recorded(
    connection: &Connection,
    project: ProjectId,
) -> Result<Option<ProjectMetadata>, PresentationError> {
    let text: Option<String> = connection
        .query_row(
            "SELECT presentation FROM project_presentation WHERE project=?1",
            [project.to_string()],
            |row| row.get(0),
        )
        .optional()?;
    text.map(|text| {
        let metadata: ProjectMetadata =
            serde_json::from_str(&text).map_err(PresentationError::Decode)?;
        if metadata.project != project {
            return Err(PresentationError::Identity(project));
        }
        Ok(metadata)
    })
    .transpose()
}

#[cfg(test)]
mod tests;
