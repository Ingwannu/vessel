//! Protocol 10's project calls (track B, ADR 0024): routing only; the work is `crate::workspace`'s.

#![expect(
    clippy::unused_async,
    reason = "the calls answer `unsupported` until their slice is built; each becomes a real call"
)]

use vessel_proto::{Ack, ErrorCode, WireError};

use super::{Connection, NodeService};

impl NodeService {
    /// `project.list`.
    pub(crate) async fn list_projects(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::ListProjects,
    ) -> Result<vessel_proto::workspace::ProjectList, WireError> {
        Err(not_built("project.list"))
    }

    /// `project.get`.
    pub(crate) async fn get_project(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::GetProject,
    ) -> Result<vessel_proto::workspace::ProjectInfo, WireError> {
        Err(not_built("project.get"))
    }

    /// `project.add`.
    pub(crate) async fn add_project(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::AddProject,
    ) -> Result<vessel_proto::workspace::ProjectInfo, WireError> {
        Err(not_built("project.add"))
    }

    /// `project.clone`.
    pub(crate) async fn clone_project(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::CloneProject,
    ) -> Result<vessel_proto::workspace::ProjectInfo, WireError> {
        Err(not_built("project.clone"))
    }

    /// `project.init`.
    pub(crate) async fn init_project(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::InitProject,
    ) -> Result<vessel_proto::workspace::ProjectInfo, WireError> {
        Err(not_built("project.init"))
    }

    /// `project.remove`.
    pub(crate) async fn remove_project(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::RemoveProject,
    ) -> Result<Ack, WireError> {
        Err(not_built("project.remove"))
    }

    /// `project.commands`.
    pub(crate) async fn project_commands(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::GetProjectCommands,
    ) -> Result<vessel_proto::workspace::ProjectCommands, WireError> {
        Err(not_built("project.commands"))
    }

    /// `project.approve`.
    pub(crate) async fn approve_commands(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::ApproveCommands,
    ) -> Result<vessel_proto::workspace::ProjectCommands, WireError> {
        Err(not_built("project.approve"))
    }
}

fn not_built(method: &str) -> WireError {
    WireError::new(
        ErrorCode::Unsupported,
        format!("`{method}` is not built on this node yet"),
    )
}
