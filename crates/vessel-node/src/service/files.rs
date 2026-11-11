//! Protocol 10's file calls (track B, ADR 0024): routing only; the work is `crate::workspace`'s.

#![expect(
    clippy::unused_async,
    reason = "the calls answer `unsupported` until their slice is built; each becomes a real call"
)]

use vessel_proto::{Ack, ErrorCode, WireError};

use super::{Connection, NodeService};

impl NodeService {
    /// `file.list`.
    pub(crate) async fn list_dir(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::ListDir,
    ) -> Result<vessel_proto::workspace::DirListing, WireError> {
        Err(not_built("file.list"))
    }

    /// `file.read`.
    pub(crate) async fn read_file(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::ReadFile,
    ) -> Result<vessel_proto::workspace::FileRead, WireError> {
        Err(not_built("file.read"))
    }

    /// `file.write`.
    pub(crate) async fn write_file(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::WriteFile,
    ) -> Result<vessel_proto::workspace::FileVersion, WireError> {
        Err(not_built("file.write"))
    }

    /// `file.create_dir`.
    pub(crate) async fn create_dir(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::CreateDir,
    ) -> Result<Ack, WireError> {
        Err(not_built("file.create_dir"))
    }

    /// `file.rename`.
    pub(crate) async fn rename_path(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::RenamePath,
    ) -> Result<Ack, WireError> {
        Err(not_built("file.rename"))
    }

    /// `file.delete`.
    pub(crate) async fn delete_path(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::DeletePath,
    ) -> Result<Ack, WireError> {
        Err(not_built("file.delete"))
    }

    /// `file.find`.
    pub(crate) async fn find_files(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::FindFiles,
    ) -> Result<vessel_proto::workspace::FoundFiles, WireError> {
        Err(not_built("file.find"))
    }

    /// `file.grep`.
    pub(crate) async fn search_files(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::SearchFiles,
    ) -> Result<vessel_proto::workspace::GrepResult, WireError> {
        Err(not_built("file.grep"))
    }
}

fn not_built(method: &str) -> WireError {
    WireError::new(
        ErrorCode::Unsupported,
        format!("`{method}` is not built on this node yet"),
    )
}
