//! Protocol 11's saved-prompt calls (ADR 0033): who may write one, and
//! whether what a session started from one runs with holds together (as
//! `session.create` would take it, its place in the prompt's project).
//! Names, the node's bound and revisions are `automation::prompts`' rules.

use chrono::Utc;
use vessel_proto::automation::{
    CreatePrompt, DeletePrompt, ListPrompts, PromptList, SavedPrompt, UpdatePrompt,
};
use vessel_proto::workspace::SessionPlace;
use vessel_proto::{Ack, ErrorCode, ProjectId, SessionSpec, WireError};

use super::projects::wire;
use super::{Connection, NodeService};
use crate::principal::Principal;
use crate::profiles::launch::creator;

/// Whether `principal` may write saved prompts: a person on this node.
fn writer(principal: &Principal) -> Result<(), WireError> {
    match principal {
        Principal::Owner | Principal::Device { .. } => Ok(()),
        // A saved prompt is later sent by a person who may not read it
        // again: an agent able to change one could put its words in that
        // person's mouth.
        Principal::Agent { .. } => Err(WireError::new(
            ErrorCode::Forbidden,
            "a saved prompt is a person's text: an agent may read saved prompts, not write them",
        )),
        Principal::Delegated { .. } => Err(WireError::new(
            ErrorCode::Forbidden,
            "saved prompts are written on their own node, not through another",
        )),
    }
}

impl NodeService {
    pub(crate) async fn list_prompts(
        &self,
        _: &Connection,
        request: ListPrompts,
    ) -> Result<PromptList, WireError> {
        if let Some(project) = request.project {
            self.project_exists(project).await?;
        }
        self.prompts
            .list(request.project)
            .await
            .map(|prompts| PromptList { prompts })
            .map_err(|error| error.to_wire())
    }

    pub(crate) async fn create_prompt(
        &self,
        connection: &Connection,
        request: CreatePrompt,
    ) -> Result<SavedPrompt, WireError> {
        let principal = connection.principal();
        writer(principal)?;
        if let Some(project) = request.project {
            self.project_exists(project).await?;
        }
        if let Some(spec) = &request.session {
            self.check_prompt_spec(principal, spec, request.project)
                .await?;
        }
        self.prompts
            .create(request, creator(principal), Utc::now())
            .await
            .map_err(|error| error.to_wire())
    }

    pub(crate) async fn update_prompt(
        &self,
        connection: &Connection,
        request: UpdatePrompt,
    ) -> Result<SavedPrompt, WireError> {
        let principal = connection.principal();
        writer(principal)?;
        let current = self
            .prompts
            .get(request.prompt)
            .await
            .map_err(|error| error.to_wire())?;
        if let Some(spec) = &request.session {
            self.check_prompt_spec(principal, spec, current.project)
                .await?;
        }
        self.prompts
            .update(request, creator(principal), Utc::now())
            .await
            .map_err(|error| error.to_wire())
    }

    pub(crate) async fn delete_prompt(
        &self,
        connection: &Connection,
        request: DeletePrompt,
    ) -> Result<Ack, WireError> {
        writer(connection.principal())?;
        self.prompts
            .delete(request.prompt)
            .await
            .map(|()| Ack)
            .map_err(|error| error.to_wire())
    }

    async fn project_exists(&self, project: ProjectId) -> Result<(), WireError> {
        self.workspaces
            .get_project(project, None, false)
            .await
            .map(|_info| ())
            .map_err(|error| wire(&error))
    }

    /// What a session started from a saved prompt runs with: as a new
    /// session's spec is checked (`check_new_spec`), and its place, if
    /// any, in the prompt's project (a machine's prompt has none: which
    /// project would it be?).
    async fn check_prompt_spec(
        &self,
        principal: &Principal,
        spec: &SessionSpec,
        project: Option<ProjectId>,
    ) -> Result<(), WireError> {
        if let Some(place) = spec.place.as_deref() {
            let placed_in = match place {
                SessionPlace::Project { project, .. } => *project,
                SessionPlace::Worktree { worktree } => {
                    self.workspaces
                        .worktree(*worktree)
                        .map_err(|error| wire(&error))?
                        .project
                }
            };
            if Some(placed_in) != project {
                return Err(WireError::new(
                    ErrorCode::InvalidRequest,
                    "a session started from a saved prompt runs in the prompt's project: its \
                     place must be in it, and a machine's prompt has no place",
                ));
            }
        }
        self.check_new_spec(principal, spec).await
    }
}
