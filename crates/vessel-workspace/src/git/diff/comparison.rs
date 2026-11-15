//! Comparison reads own their private index through the complete result.
//!
//! [Decision Log]
//! - Purpose: summary and file reads must give identical All semantics.
//! - Existing assets: protected git runner, bounded summary/hunk readers and
//!   the private-index lease; no second parser or command executor is needed.
//! - Choice: one query context, one preparation and one finish boundary.
//! - Why: cleanup must run after failed/cancelled reads too. The node owns the
//!   executing task until this future finishes, even when a client disconnects.
//! - Cost: each body read prepares its own bounded index until shared snapshot
//!   caching lands. No stronger atomic filesystem snapshot is claimed.

use std::path::Path;

use vessel_proto::workspace::{
    DiffSummary, FileDiff, GitComparison, GitHunkOffset, GitWhitespace, RelPath, WorktreeBase,
};

use super::ComparisonError;
use crate::git::Git;

/// Resolved node-owned paths and the exact selected comparison.
/// The node revalidates its workspace root before constructing this context.
#[derive(Debug, Clone, Copy)]
pub struct Comparison<'a> {
    /// The validated checkout, never a path supplied in an RPC.
    pub directory: &'a Path,
    /// The selected sides; committed never falls back to All.
    pub kind: &'a GitComparison,
    /// Registered project/worktree default when the query names none.
    pub base: Option<&'a WorktreeBase>,
    /// Both summaries and bodies use this same preference.
    pub whitespace: GitWhitespace,
    /// Absolute scratch directory owned/prepared by the node.
    pub scratch: &'a Path,
}

impl Git {
    /// Read changed files, finishing private-index cleanup before returning.
    /// `None` means there is no comparison base, not an empty change list.
    /// The caller must own this operation until completion (node task tracker);
    /// cancellation stops its commands, never abandons its cleanup future.
    ///
    /// # Errors
    /// Preparation, protected git reads and cleanup keep their typed sources.
    pub async fn comparison_changes(
        &self,
        query: Comparison<'_>,
    ) -> Result<Option<DiffSummary>, ComparisonError> {
        let Some(prepared) = self
            .prepare_comparison(query.directory, query.kind, query.base, query.scratch)
            .await?
        else {
            return Ok(None);
        };
        let read = prepared
            .changes(self, query.directory, query.whitespace)
            .await;
        prepared.finish(read).await.map(Some)
    }

    /// Read a bounded file page and await cleanup even after cancellation.
    /// The node must retain task ownership as for `comparison_changes`.
    ///
    /// # Errors
    /// As [`comparison_changes`](Self::comparison_changes), plus file reads.
    pub async fn comparison_file(
        &self,
        query: Comparison<'_>,
        path: &RelPath,
        offset: GitHunkOffset,
    ) -> Result<Option<FileDiff>, ComparisonError> {
        let Some(prepared) = self
            .prepare_comparison(query.directory, query.kind, query.base, query.scratch)
            .await?
        else {
            return Ok(None);
        };
        let read = prepared
            .file(self, query.directory, path, query.whitespace, offset)
            .await;
        prepared.finish(read).await.map(Some)
    }
}

#[cfg(test)]
mod tests;
