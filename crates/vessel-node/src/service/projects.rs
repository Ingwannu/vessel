//! Protocol 10's project calls (track B, ADR 0027): routing only; the work
//! is `crate::workspace`'s.

use vessel_proto::workspace::{
    AddProject, ApproveCommands, CloneProject, GetProject, GetProjectCommands, InitProject,
    ListProjects, ProjectCommands, ProjectInfo, ProjectList, RemoveProject,
};
use vessel_proto::{Ack, WireError};

use super::{Connection, NodeService};
use crate::principal::Principal;
use crate::waits::Waiter;
use crate::workspace::{MAX_WAIT_SECS, WorkspaceError};

/// A workspace failure, on the wire.
pub(super) fn wire(error: &WorkspaceError) -> WireError {
    error.wire()
}

impl NodeService {
    /// `project.list`.
    pub(crate) async fn list_projects(
        &self,
        _: &Connection,
        _: ListProjects,
    ) -> Result<ProjectList, WireError> {
        Ok(self.workspaces.list_projects().await)
    }

    /// `project.get`: with `changed_from`, a long poll inside the caller's
    /// wait shares (as `session.read`'s); a forwarded call answers at once,
    /// so a home's link is never held by a member's poll.
    pub(crate) async fn get_project(
        &self,
        connection: &Connection,
        request: GetProject,
    ) -> Result<ProjectInfo, WireError> {
        let mut wait = request.changed_from.map(|_| MAX_WAIT_SECS);
        if matches!(connection.principal(), Principal::Delegated { .. }) {
            wait = None;
        }
        let here = self.links.node();
        let shares = Waiter::of(connection.principal(), here).shares(here);
        let _slot = self.wait_slot(connection, shares, &mut wait);
        self.workspaces
            .get_project(request.project, request.changed_from, wait.is_some())
            .await
            .map_err(|error| wire(&error))
    }

    /// `project.add`.
    pub(crate) async fn add_project(
        &self,
        _: &Connection,
        request: AddProject,
    ) -> Result<ProjectInfo, WireError> {
        self.workspaces
            .add_project(request.path, request.name)
            .await
            .map_err(|error| wire(&error))
    }

    /// `project.clone`: answers with the project `cloning`.
    pub(crate) async fn clone_project(
        &self,
        _: &Connection,
        request: CloneProject,
    ) -> Result<ProjectInfo, WireError> {
        self.workspaces
            .clone_project(request.url, request.parent, request.folder, request.name)
            .await
            .map_err(|error| wire(&error))
    }

    /// `project.init`.
    pub(crate) async fn init_project(
        &self,
        _: &Connection,
        request: InitProject,
    ) -> Result<ProjectInfo, WireError> {
        self.workspaces
            .init_project(request.path, request.name)
            .await
            .map_err(|error| wire(&error))
    }

    /// `project.remove`: forgets it; its files stay.
    pub(crate) async fn remove_project(
        &self,
        _: &Connection,
        request: RemoveProject,
    ) -> Result<Ack, WireError> {
        let live = self.sessions.live_folders();
        self.workspaces
            .remove_project(request.project, &live)
            .await
            .map(|()| Ack)
            .map_err(|error| wire(&error))
    }

    /// `project.commands`.
    pub(crate) async fn project_commands(
        &self,
        _: &Connection,
        request: GetProjectCommands,
    ) -> Result<ProjectCommands, WireError> {
        self.workspaces
            .commands(request.project, request.base.as_ref())
            .await
            .map_err(|error| wire(&error))
    }

    /// `project.approve`: agents are refused before this runs (`guard`).
    pub(crate) async fn approve_commands(
        &self,
        _: &Connection,
        request: ApproveCommands,
    ) -> Result<ProjectCommands, WireError> {
        self.workspaces
            .approve(request.project, request.digest, request.approved)
            .await
            .map_err(|error| wire(&error))
    }
}
