//! Project/clone adapters keep the existing registry and clone authority on the node.
use super::output::{self, Row};
use crate::{
    args::{
        ProjectCommand, Target,
        parity::{Clone, CloneProtocol, Presentation},
    },
    connect::connect,
    error::CliError,
};
use serde::Serialize;
use std::{path::PathBuf, time::Duration};
use vessel_proto::{
    ProjectId,
    workspace::{
        AddProject, CloneProject, CloneUrl, GetProject, ListProjects, ProjectInfo, ProjectName,
        ProjectState,
    },
};
/// A clone can outlast one RPC, but not the command's explicit finite budget.
const CLONE_DEADLINE: Duration = Duration::from_secs(10 * 60);

pub(crate) async fn project(
    target: &Target,
    presentation: Presentation,
    command: ProjectCommand,
) -> Result<(), CliError> {
    match command {
        ProjectCommand::List => {
            let client = connect(target).await?.client;
            let result = client.call(ListProjects {}).await?;
            if result.more {
                return Err(CliError::ParityBlocked(
                    "project list is truncated by this node; a complete registry snapshot is required",
                ));
            }
            let rows = result
                .projects
                .into_iter()
                .map(ProjectRow::from)
                .collect::<Vec<_>>();
            output::list(presentation, &rows).await
        }
        ProjectCommand::Create { path } => {
            let path = project_path(target, path)?;
            let client = connect(target).await?.client;
            let project = client.call(AddProject { path, name: None }).await?;
            let row = ProjectRow::from(project);
            output::record(presentation, &row).await
        }
        ProjectCommand::Rename { .. } | ProjectCommand::Delete { .. } => {
            Err(CliError::ParityBlocked(
                "project rename/reset/cascading delete awaits W11's reviewed registry contract",
            ))
        }
        legacy @ (ProjectCommand::Add { .. }
        | ProjectCommand::Clone { .. }
        | ProjectCommand::Init { .. }
        | ProjectCommand::Remove { .. }
        | ProjectCommand::Commands { .. }
        | ProjectCommand::Approve { .. }) => crate::workspace::project(target, legacy).await,
    }
}
fn project_path(target: &Target, path: Option<PathBuf>) -> Result<PathBuf, CliError> {
    let (resolved, _book) = crate::nodes::resolve(target)?;
    // A selected destination can be reached through a loopback tunnel. Only
    // the resolver's implicit-local origin gives CLI cwd its local meaning.
    let implicit_local =
        target.node.is_none() && matches!(resolved.origin, vessel_client::book::Origin::Default);
    match path {
        Some(path) if path.is_absolute() => Ok(path),
        Some(path) if implicit_local => Ok(std::path::absolute(path)?),
        Some(_) => Err(CliError::Usage(
            "selected node project/clone paths must be absolute on that node",
        )),
        None if implicit_local => Ok(std::env::current_dir()?),
        None => Err(CliError::Usage(
            "project create requires an explicit absolute path with --host/--node or a selected default",
        )),
    }
}

pub(super) async fn clone(
    target: &Target,
    presentation: Presentation,
    args: Clone,
) -> Result<(), CliError> {
    let url = remote(&args.repo, args.protocol)?;
    let parent = project_path(target, Some(args.dir))?;
    let client = connect(target).await?.client;
    let project = client
        .call(CloneProject {
            url,
            parent,
            folder: None,
            name: None,
        })
        .await?;
    let id = project.id;
    let ready = tokio::time::timeout(CLONE_DEADLINE, finish_clone(&client, project))
        .await
        .map_err(|source| CliError::ParityCloneTimeout {
            project: id,
            source,
        })??;
    let result = Cloned {
        repo: RepoDisplay(args.repo),
        checkout_path: ready.root,
        project_id: ready.id,
        project_name: ready.name,
    };
    output::single(presentation, &ready.id.to_string(), &result).await
}
async fn finish_clone(
    client: &vessel_client::NodeClient,
    mut project: ProjectInfo,
) -> Result<ProjectInfo, CliError> {
    loop {
        match &project.state {
            ProjectState::Ready => return Ok(project),
            ProjectState::Cloning { .. } => {
                project = client
                    .call(GetProject {
                        project: project.id,
                        changed_from: Some(project.revision),
                    })
                    .await?;
            }
            ProjectState::CloneFailed { reason, message } => {
                return Err(CliError::ParityCloneFailed {
                    project: project.id,
                    reason: *reason,
                    message: message.clone(),
                });
            }
            ProjectState::Missing | ProjectState::Unknown => {
                return Err(CliError::ParityBlocked(
                    "clone did not produce a usable project directory",
                ));
            }
        }
    }
}
fn remote(repo: &str, protocol: Option<CloneProtocol>) -> Result<CloneUrl, CliError> {
    if repo.contains("://") || repo.contains('@') {
        return repo.parse().map_err(CliError::ParityCloneUrl);
    }
    let (owner, name) = repo.split_once('/').ok_or(CliError::Usage(
        "repository must be a complete remote or owner/repo",
    ))?;
    let valid = |part: &str| {
        !part.is_empty()
            && part != "."
            && part != ".."
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    };
    if !valid(owner) || !valid(name) {
        return Err(CliError::Usage(
            "owner/repo must contain one bounded repository path",
        ));
    }
    let text = match protocol {
        Some(CloneProtocol::Https) => format!("https://github.com/{owner}/{name}.git"),
        Some(CloneProtocol::Ssh) => format!("git@github.com:{owner}/{name}.git"),
        None => {
            return Err(CliError::Usage(
                "--protocol https|ssh is required for owner/repo shorthand",
            ));
        }
    };
    text.parse().map_err(CliError::ParityCloneUrl)
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Cloned {
    repo: RepoDisplay,
    checkout_path: PathBuf,
    project_id: ProjectId,
    project_name: ProjectName,
}
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ProjectKind {
    Git,
    Directory,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectRow {
    project_id: ProjectId,
    name: ProjectName,
    kind: ProjectKind,
    path: PathBuf,
}
impl From<ProjectInfo> for ProjectRow {
    fn from(project: ProjectInfo) -> Self {
        Self {
            project_id: project.id,
            name: project.name,
            kind: if project.git {
                ProjectKind::Git
            } else {
                ProjectKind::Directory
            },
            path: project.root,
        }
    }
}
impl Row for ProjectRow {
    fn id(&self) -> String {
        self.project_id.to_string()
    }
    fn headers() -> &'static [&'static str] {
        &["PROJECT ID", "NAME", "KIND", "PATH"]
    }
    fn cells(&self) -> Vec<String> {
        vec![
            self.project_id.to_string(),
            self.name.to_string(),
            match self.kind {
                ProjectKind::Git => "git",
                ProjectKind::Directory => "directory",
            }
            .to_owned(),
            self.path.display().to_string(),
        ]
    }
}
#[derive(Serialize)]
#[serde(transparent)]
struct RepoDisplay(String);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_remotes_keep_their_transport_and_shorthand_requires_a_choice() {
        assert!(remote("owner/repo", None).is_err());
        assert_eq!(
            remote("owner/repo", Some(CloneProtocol::Ssh))
                .unwrap()
                .as_str(),
            "git@github.com:owner/repo.git"
        );
        assert!(remote("https://user:password@example.invalid/repo", None).is_err());
        assert!(remote("owner/../repo", Some(CloneProtocol::Https)).is_err());
    }
}
