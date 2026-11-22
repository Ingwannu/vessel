//! Shared lifecycle for independently provisioned workspaces (ADR 0082).
//! Callers authorize their own operation before passing exact identities here.

mod archive;
mod provision;
mod recovery;

pub(crate) use archive::{ArchiveResult, WorkspaceActors};
pub(crate) use provision::Provisioned;
pub(crate) use recovery::ReconciledProvision;
