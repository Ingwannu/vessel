//! Borrowed rows and canonical pane ownership lookup, without route guessing.
use super::{Entry, RegistryData, RegistryPhase, RegistryProjection};
use crate::{
    connection::ConnectionContext,
    layout::{LayoutKey, LookupProblem, WorkspaceLocation, WorkspaceLookup},
};
use vessel_proto::{
    NodeId, ProjectId, SessionId, WorkspaceId,
    registry::{
        WorkspaceBacking, WorkspaceDescriptor, WorkspaceLifecycle, WorkspaceResource,
        WorkspaceVisibility,
    },
    workspace::{BranchName, ProjectInfo, ProjectState},
};

/// Native project grouping key; a display fact, never permission to call a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProjectPlacement {
    node: NodeId,
    project: ProjectId,
}
impl ProjectPlacement {
    /// Cryptographic owner of the project identifier.
    #[must_use]
    pub const fn node(self) -> NodeId {
        self.node
    }
    /// Canonical registered directory identity.
    #[must_use]
    pub const fn project(self) -> ProjectId {
        self.project
    }
}

/// Borrow canonical values instead of retaining a second local descriptor store.
#[derive(Debug)]
pub struct WorkspaceRow<'a> {
    context: &'a ConnectionContext,
    data: &'a RegistryData,
    workspace: &'a WorkspaceDescriptor,
}
impl WorkspaceRow<'_> {
    /// The exact source admission; actual actions recheck it through `SelectedConnection`.
    #[must_use]
    pub const fn context(&self) -> &ConnectionContext {
        self.context
    }
    /// Canonical title, pin, visibility, gate and lifecycle without client overrides.
    #[must_use]
    pub const fn workspace(&self) -> &WorkspaceDescriptor {
        self.workspace
    }
    /// Reuse the sole pane/layout key rather than defining a second `WorkspaceId` key.
    #[must_use]
    pub fn key(&self) -> LayoutKey {
        LayoutKey::Workspace {
            node: self.context.node(),
            workspace: self.workspace.id,
        }
    }
    /// Project grouping never uses a path or a currently selected session.
    #[must_use]
    pub fn placement(&self) -> ProjectPlacement {
        ProjectPlacement {
            node: self.context.node(),
            project: self.workspace.project,
        }
    }
    /// Exact resource associations in this row's same complete registry snapshot.
    /// Pending, failed and archived resources remain associated; this is neither
    /// a live-member list nor evidence of role, activity, process exit or readiness.
    pub fn resources(&self) -> impl Iterator<Item = WorkspaceResource> + '_ {
        self.data
            .registry
            .memberships
            .iter()
            .filter(move |member| member.workspace == self.workspace.id)
            .map(|member| member.resource)
    }
    /// Registered project data may be incomplete independently of the registry snapshot.
    #[must_use]
    pub fn project(&self) -> Option<&ProjectInfo> {
        self.data
            .projects
            .projects
            .iter()
            .find(|project| project.id == self.workspace.project)
    }
    /// Branch display comes from the explicit backing id, never a cwd match.
    #[must_use]
    pub fn branch(&self) -> Option<&BranchName> {
        let WorkspaceBacking::Managed { worktree } = self.workspace.backing else {
            return None;
        };
        self.data
            .worktrees
            .worktrees
            .iter()
            .find(|entry| entry.id == worktree && entry.project == self.workspace.project)?
            .branch
            .as_ref()
    }
    /// The node's registered Git fact for this exact supported placement.
    /// Actual panel I/O still resolves and checks the root on the node.
    ///
    /// # Errors
    /// Absent/inconsistent catalogs and unsupported nested placement remain unknown.
    pub fn is_git(&self) -> Result<bool, LookupProblem> {
        match self.location()? {
            WorkspaceLocation::External => Err(LookupProblem::Unavailable),
            WorkspaceLocation::Root(_) => self
                .project()
                .map(|project| project.git)
                .ok_or(LookupProblem::Unavailable),
        }
    }

    /// Default home/sidebar visibility, preserving background and archived records in data.
    #[must_use]
    pub fn visible(&self) -> bool {
        matches!(self.workspace.lifecycle, WorkspaceLifecycle::Active)
            && self.workspace.visibility == WorkspaceVisibility::Foreground
    }
    /// Only exact legacy root projections are exposed before nested-root support lands.
    ///
    /// # Errors
    /// Non-active or absent backing/catalog metadata cannot authorize a panel root.
    pub fn location(&self) -> Result<WorkspaceLocation, LookupProblem> {
        location(self.data, self.workspace)
    }
}

impl RegistryProjection {
    /// All canonical rows for one explicit admission; callers choose their own filter/order.
    ///
    /// # Errors
    /// Loading, unsupported and retired admissions never look like an empty list.
    pub fn rows(
        &self,
        context: &ConnectionContext,
    ) -> Result<impl Iterator<Item = WorkspaceRow<'_>>, LookupProblem> {
        let entry = self.admitted(context)?;
        let data = entry.data.as_ref().ok_or(LookupProblem::NotHydrated)?;
        Ok(data
            .registry
            .workspaces
            .iter()
            .map(move |workspace| WorkspaceRow {
                context: &entry.context,
                data,
                workspace,
            }))
    }

    /// The exact source of the metadata selected for a node lookup.
    /// The app validates that source's canonical weak guard before using lookup facts.
    /// This never selects a client or an action route.
    ///
    /// # Errors
    /// Missing or conflicting complete snapshots have no usable source.
    pub fn read_context(&self, node: NodeId) -> Result<&ConnectionContext, LookupProblem> {
        self.node_entry(node).map(|entry| &entry.context)
    }

    // This selects metadata only. It never chooses a client/alias for an action.
    // Conflicting equal-revision snapshots are unavailable rather than first-wins.
    fn node_entry(&self, node: NodeId) -> Result<&Entry, LookupProblem> {
        let mut selected: Option<&Entry> = None;
        let mut unavailable = false;
        for entry in self
            .hosts
            .values()
            .filter(|entry| entry.context.node() == node)
        {
            if entry.phase != RegistryPhase::Ready {
                unavailable |= entry.phase != RegistryPhase::Loading;
                continue;
            }
            let data = entry.data.as_ref().ok_or(LookupProblem::Unavailable)?;
            if let Some(previous) = selected {
                let previous_data = previous.data.as_ref().ok_or(LookupProblem::Unavailable)?;
                match data.registry.revision.cmp(&previous_data.registry.revision) {
                    std::cmp::Ordering::Equal if data != previous_data => {
                        return Err(LookupProblem::Unavailable);
                    }
                    std::cmp::Ordering::Less | std::cmp::Ordering::Equal => continue,
                    std::cmp::Ordering::Greater => {}
                }
            }
            selected = Some(entry);
        }
        selected.ok_or(if unavailable {
            LookupProblem::Unavailable
        } else {
            LookupProblem::NotHydrated
        })
    }
}

impl WorkspaceLookup for RegistryProjection {
    fn session_workspace(
        &self,
        node: NodeId,
        session: SessionId,
    ) -> Result<Option<WorkspaceId>, LookupProblem> {
        let data = self
            .node_entry(node)?
            .data
            .as_ref()
            .ok_or(LookupProblem::NotHydrated)?;
        Ok(data.registry.owner(WorkspaceResource::Session { session }))
    }
    fn workspace_location(
        &self,
        node: NodeId,
        workspace: WorkspaceId,
    ) -> Result<WorkspaceLocation, LookupProblem> {
        let data = self
            .node_entry(node)?
            .data
            .as_ref()
            .ok_or(LookupProblem::NotHydrated)?;
        let workspace = data
            .registry
            .workspaces
            .iter()
            .find(|entry| entry.id == workspace)
            .ok_or(LookupProblem::Unavailable)?;
        location(data, workspace)
    }
}

fn location(
    data: &RegistryData,
    workspace: &WorkspaceDescriptor,
) -> Result<WorkspaceLocation, LookupProblem> {
    if !matches!(workspace.lifecycle, WorkspaceLifecycle::Active) {
        return Err(LookupProblem::Unavailable);
    }
    match workspace.backing {
        WorkspaceBacking::Local => {
            let project = data
                .projects
                .projects
                .iter()
                .find(|project| project.id == workspace.project)
                .ok_or(LookupProblem::Unavailable)?;
            if !matches!(project.state, ProjectState::Ready) {
                return Err(LookupProblem::Unavailable);
            }
            if project.root != workspace.path {
                return Ok(WorkspaceLocation::External);
            }
        }
        WorkspaceBacking::Managed { worktree } => {
            let project = data
                .projects
                .projects
                .iter()
                .find(|project| project.id == workspace.project)
                .ok_or(LookupProblem::Unavailable)?;
            if !project.git || !matches!(project.state, ProjectState::Ready) {
                return Err(LookupProblem::Unavailable);
            }
            let backing = data
                .worktrees
                .worktrees
                .iter()
                .find(|entry| entry.id == worktree && entry.project == workspace.project)
                .ok_or(LookupProblem::Unavailable)?;
            if backing.path != workspace.path {
                return Ok(WorkspaceLocation::External);
            }
        }
    }
    Ok(WorkspaceLocation::Root(workspace.root()))
}
