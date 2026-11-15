//! What a project's file (`vessel.toml`) would run, read at a commit or
//! in a folder as it is, and a person's approval of that set, with what
//! it covered (protocol 11: the set last approved, shown beside a set
//! that is not; `vessel_proto::workspace::approval` says why). Nothing is
//! run here: setup and teardown are the workspace's `setup`.

use std::path::Path;
use std::sync::Arc;

use vessel_proto::ProjectId;
use vessel_proto::workspace::{
    ApprovedCommands, CommandSet, CommandsDigest, ProjectCommands, WorktreeBase,
};
use vessel_store::{ApprovalChange, MAX_APPROVED_SETS};
use vessel_workspace::project_file::{self, ProjectFile, ProjectFileError};

use crate::workspace::{State, WorkspaceError, Workspaces};

/// The most sets described lately the node remembers by digest, so an
/// approval of one keeps what it covered: a person approves what a page
/// was just shown. Each is a parsed file of at most 64 KiB.
pub(crate) const DESCRIBED: usize = 16;

impl Workspaces {
    /// What the project's file names, at `base` or in its folder as it is,
    /// and whether that set is approved.
    ///
    /// # Errors
    /// `not_found`; `invalid_request` for a base that does not resolve.
    pub(crate) async fn commands(
        &self,
        project: ProjectId,
        base: Option<&WorktreeBase>,
    ) -> Result<ProjectCommands, WorkspaceError> {
        let root = self.project_root(project)?;
        let file = match base {
            Some(base) => self.file_at(&root, base).await?,
            None => self.file_in(&root).await,
        };
        Ok(self.describe(project, file))
    }

    /// The project's file at a commit or ref, read as a blob.
    pub(crate) async fn file_at(
        &self,
        root: &Path,
        base: &WorktreeBase,
    ) -> Result<Result<Option<ProjectFile>, ProjectFileError>, WorkspaceError> {
        let git = self.git()?;
        let commit = git
            .resolve(root, base)
            .await?
            .ok_or_else(|| WorkspaceError::Invalid("the base does not name a commit".to_owned()))?;
        let bytes = git
            .read_blob(
                root,
                &commit,
                project_file::FILE_NAME,
                project_file::MAX_BYTES,
            )
            .await;
        Ok(match bytes {
            Ok(Some(bytes)) => ProjectFile::parse(&bytes).map(Some),
            Ok(None) => Ok(None),
            Err(vessel_workspace::GitError::OutputLimit) => Err(ProjectFileError::TooLarge),
            Err(error) => return Err(error.into()),
        })
    }

    /// The project's file in a folder as it is.
    /// Only a regular file inside the folder is read, and only up to the
    /// bound (`ProjectFile::read_in`): an agent can write in a project's
    /// folder and in its own worktree.
    pub(crate) async fn file_in(
        &self,
        folder: &Path,
    ) -> Result<Option<ProjectFile>, ProjectFileError> {
        let folder = folder.to_path_buf();
        tokio::task::spawn_blocking(move || ProjectFile::read_in(&folder))
            .await
            .unwrap_or_else(|stopped| {
                Err(ProjectFileError::Unreadable(std::io::Error::other(stopped)))
            })
    }

    /// The wire's form of a parsed file, with its approval.
    pub(crate) fn describe(
        &self,
        project: ProjectId,
        file: Result<Option<ProjectFile>, ProjectFileError>,
    ) -> ProjectCommands {
        match file {
            // Said on the wire as text, here only.
            Err(error) => ProjectCommands::Invalid {
                message: error.to_string(),
            },
            Ok(None) => ProjectCommands::None,
            Ok(Some(file)) if file.is_empty() => ProjectCommands::None,
            Ok(Some(file)) => {
                let digest = file.digest();
                let set = CommandSet {
                    setup: file.setup,
                    teardown: file.teardown,
                    scripts: file.scripts,
                    terminals: file.terminals,
                    service_ports: file.service_ports,
                };
                let (approved, last_approved) = {
                    let mut state = self.state();
                    state.remember(&digest, &set);
                    let approved = state.approvals.contains(&(project, digest.clone()));
                    let last = (!approved)
                        .then(|| state.approved_sets.get(&project)?.first().cloned())
                        .flatten();
                    (approved, last.map(Box::new))
                };
                ProjectCommands::Listed {
                    approved,
                    setup: set.setup,
                    teardown: set.teardown,
                    scripts: set.scripts,
                    terminals: set.terminals,
                    service_ports: set.service_ports,
                    digest,
                    last_approved,
                }
            }
        }
    }

    /// Whether a person approved `digest` for `project`.
    pub(crate) fn approved(&self, project: ProjectId, digest: &CommandsDigest) -> bool {
        self.state().approvals.contains(&(project, digest.clone()))
    }

    /// Approve or withdraw a set; the folder's commands after. An
    /// approval keeps the set it covers when the node described that
    /// digest lately (a page approves what it was just shown).
    ///
    /// # Errors
    /// `not_found`.
    pub(crate) async fn approve(
        &self,
        project: ProjectId,
        digest: CommandsDigest,
        approved: bool,
    ) -> Result<ProjectCommands, WorkspaceError> {
        let root = self.project_root(project)?;
        let at = chrono::Utc::now();
        let covered = approved
            .then(|| self.state().described_as(&digest))
            .flatten();
        let change = if approved {
            ApprovalChange::Approved {
                at,
                commands: covered.as_deref().cloned().map(Box::new),
            }
        } else {
            ApprovalChange::Withdrawn
        };
        self.store
            .set_approval(project, digest.clone(), change)
            .await?;
        {
            let mut state = self.state();
            let sets = state.approved_sets.entry(project).or_default();
            // Approved again, it moves to the front with the newest time,
            // keeping what it covered if the node no longer has it in mind
            // (as the store keeps it); withdrawn, it goes.
            let mut known = None;
            sets.retain(|set| {
                let same = set.digest == digest;
                if same {
                    known = Some(set.commands.clone());
                }
                !same
            });
            let commands = covered.map(Arc::unwrap_or_clone).or(known);
            if approved && let Some(commands) = commands {
                sets.insert(
                    0,
                    ApprovedCommands {
                        commands,
                        digest: digest.clone(),
                        approved_at: at,
                    },
                );
                sets.truncate(MAX_APPROVED_SETS);
            }
            if approved {
                let _new = state.approvals.insert((project, digest));
            } else {
                let _was = state.approvals.remove(&(project, digest));
            }
        }
        let file = self.file_in(&root).await;
        Ok(self.describe(project, file))
    }
}

impl State {
    /// Remember `set` as described under `digest`, newest last, so an
    /// approval of it keeps what it covers.
    fn remember(&mut self, digest: &CommandsDigest, set: &CommandSet) {
        if let Some(at) = self.described.iter().position(|(seen, _)| seen == digest) {
            if let Some(entry) = self.described.remove(at) {
                self.described.push_back(entry);
            }
            return;
        }
        if self.described.len() >= DESCRIBED {
            let _oldest = self.described.pop_front();
        }
        self.described
            .push_back((digest.clone(), Arc::new(set.clone())));
    }

    /// The set described lately under `digest`.
    fn described_as(&self, digest: &CommandsDigest) -> Option<Arc<CommandSet>> {
        self.described
            .iter()
            .find(|(seen, _)| seen == digest)
            .map(|(_, set)| Arc::clone(set))
    }
}
