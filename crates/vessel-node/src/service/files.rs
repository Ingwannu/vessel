//! Protocol 10's file calls (track B, ADR 0027): routing only; the work is
//! `crate::workspace`'s.

use vessel_proto::workspace::{
    CreateDir, DeletePath, DirListing, FileRead, FileVersion, FindFiles, FoundFiles, GrepResult,
    ListDir, ReadFile, RenamePath, SearchFiles, WriteFile,
};
use vessel_proto::{Ack, WireError};

use super::projects::wire;
use super::{Connection, NodeService};
use crate::transfers::Holder;

impl NodeService {
    /// `file.list`.
    pub(crate) async fn list_dir(
        &self,
        _: &Connection,
        request: ListDir,
    ) -> Result<DirListing, WireError> {
        self.workspaces
            .list_dir(request.root, &request.dir, request.ignored)
            .await
            .map_err(|error| wire(&error))
    }

    /// `file.read`.
    pub(crate) async fn read_file(
        &self,
        _: &Connection,
        request: ReadFile,
    ) -> Result<FileRead, WireError> {
        self.workspaces
            .read_file(request.root, &request.path, request.offset, request.limit)
            .await
            .map_err(|error| wire(&error))
    }

    /// `file.write`.
    pub(crate) async fn write_file(
        &self,
        _: &Connection,
        request: WriteFile,
    ) -> Result<FileVersion, WireError> {
        self.run_file_work(self.workspaces.write_file(
            request.root,
            &request.path,
            request.text,
            request.expect,
        ))
        .await
    }

    /// `file.create_dir`.
    pub(crate) async fn create_dir(
        &self,
        _: &Connection,
        request: CreateDir,
    ) -> Result<Ack, WireError> {
        self.run_file_work(self.workspaces.create_dir(request.root, &request.path))
            .await
            .map(|()| Ack)
    }

    /// `file.rename`.
    pub(crate) async fn rename_path(
        &self,
        _: &Connection,
        request: RenamePath,
    ) -> Result<Ack, WireError> {
        self.run_file_work(
            self.workspaces
                .rename_path(request.root, &request.from, &request.to),
        )
        .await
        .map(|()| Ack)
    }

    /// `file.delete`.
    pub(crate) async fn delete_path(
        &self,
        _: &Connection,
        request: DeletePath,
    ) -> Result<Ack, WireError> {
        self.run_file_work(self.workspaces.delete_path(
            request.root,
            &request.path,
            request.recursive,
        ))
        .await
        .map(|()| Ack)
    }

    /// `file.find`.
    pub(crate) async fn find_files(
        &self,
        connection: &Connection,
        request: FindFiles,
    ) -> Result<FoundFiles, WireError> {
        let caller = Holder::of(connection.principal());
        self.workspaces
            .find_files(
                caller,
                request.root,
                &request.query,
                request.key,
                request.limit,
            )
            .await
            .map_err(|error| wire(&error))
    }

    /// `file.grep`.
    pub(crate) async fn search_files(
        &self,
        _: &Connection,
        request: SearchFiles,
    ) -> Result<GrepResult, WireError> {
        self.workspaces
            .search_files(
                request.root,
                &request.query,
                request.mode,
                request.case,
                request.dir.as_ref(),
                request.limit,
            )
            .await
            .map_err(|error| wire(&error))
    }
}
