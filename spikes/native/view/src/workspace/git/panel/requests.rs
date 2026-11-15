//! Read identity for a pane: transport replies may outlive repository, mode,
//! base and whitespace changes. Reuse the accepted monotonic request issuer.

use super::{Comparison, RepositoryKey};
use crate::workspace::git::{Reading, RequestKey};
use crate::workspace::problem::Problem;
use std::collections::HashMap;
use vessel_proto::workspace::{DiffSummary, FileDiff, GitStatus, RelPath};

/// A status belongs to a repository, independently of comparison selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusTicket {
    repository: RepositoryKey,
    key: RequestKey,
}

impl StatusTicket {
    /// The link routes only to this actual node/root.
    #[must_use]
    pub const fn repository(self) -> RepositoryKey {
        self.repository
    }
}

/// A summary belongs to the full normalized comparison identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparisonTicket {
    repository: RepositoryKey,
    comparison: Comparison,
    key: RequestKey,
}

impl ComparisonTicket {
    /// The link routes only to this actual node/root.
    #[must_use]
    pub const fn repository(&self) -> RepositoryKey {
        self.repository
    }

    /// A legacy node must not substitute All for unsupported Committed.
    #[must_use]
    pub const fn comparison(&self) -> &Comparison {
        &self.comparison
    }
}

/// File body identity also includes its relative path; counts are not a version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTicket {
    comparison: ComparisonTicket,
    path: RelPath,
}

impl FileTicket {
    /// Original query identity, echoed intact through the link.
    #[must_use]
    pub const fn comparison(&self) -> &ComparisonTicket {
        &self.comparison
    }

    /// Only this file's body may be settled.
    #[must_use]
    pub const fn path(&self) -> &RelPath {
        &self.path
    }
}

/// A pane's read store. `RequestKeys` must live above it, across pane recreation.
#[derive(Debug)]
pub struct PanelReads {
    repository: RepositoryKey,
    comparison: Comparison,
    status: Reading<GitStatus>,
    summary: Reading<DiffSummary>,
    files: HashMap<RelPath, Reading<FileDiff>>,
}

impl PanelReads {
    /// A new target has no usable facts from the prior repository.
    #[must_use]
    pub fn new(repository: RepositoryKey, comparison: Comparison) -> Self {
        Self {
            repository,
            comparison,
            status: Reading::Empty,
            summary: Reading::Empty,
            files: HashMap::new(),
        }
    }

    /// Repository identity is never inferred from whichever session is selected.
    #[must_use]
    pub const fn repository(&self) -> RepositoryKey {
        self.repository
    }

    /// Current comparison, including base and whitespace.
    #[must_use]
    pub const fn comparison(&self) -> &Comparison {
        &self.comparison
    }

    /// Selecting another mode/base/filter invalidates dependent reads at once.
    pub fn select(&mut self, comparison: Comparison) {
        if self.comparison != comparison {
            self.comparison = comparison;
            self.summary = Reading::Empty;
            self.files.clear();
        }
    }

    /// Loading retains display facts but does not authorize writes from them.
    #[must_use]
    pub fn begin_status(&mut self, key: RequestKey) -> StatusTicket {
        self.status.start(key);
        StatusTicket {
            repository: self.repository,
            key,
        }
    }

    /// Unsent/full/closed settles through the same boundary as a node refusal.
    #[must_use]
    pub fn finish_status(
        &mut self,
        ticket: StatusTicket,
        answer: Result<GitStatus, Problem>,
    ) -> bool {
        ticket.repository == self.repository && self.status.accept(ticket.key, answer)
    }

    /// Begin a read of exactly the selected comparison.
    #[must_use]
    pub fn begin_summary(&mut self, key: RequestKey) -> ComparisonTicket {
        self.summary.start(key);
        ComparisonTicket {
            repository: self.repository,
            comparison: self.comparison.clone(),
            key,
        }
    }

    /// A refreshed summary may have identical counts but different text. Until
    /// node revisions prove equality, body pages must be read again; guessing
    /// freshness from paths/counts caused the old code panel's stale diff gap.
    #[must_use]
    pub fn finish_summary(
        &mut self,
        ticket: &ComparisonTicket,
        answer: Result<DiffSummary, Problem>,
    ) -> bool {
        if !self.matches(ticket) || !self.summary.accept(ticket.key, answer) {
            return false;
        }
        self.files.clear();
        true
    }

    /// A visible file owns one current body read; issuing again supersedes it.
    #[must_use]
    pub fn begin_file(&mut self, path: RelPath, key: RequestKey) -> FileTicket {
        self.files.entry(path.clone()).or_default().start(key);
        FileTicket {
            comparison: ComparisonTicket {
                repository: self.repository,
                comparison: self.comparison.clone(),
                key,
            },
            path,
        }
    }

    /// The path and echoed identity must both match, including on send failure.
    #[must_use]
    pub fn finish_file(&mut self, ticket: &FileTicket, answer: Result<FileDiff, Problem>) -> bool {
        self.matches(&ticket.comparison)
            && self
                .files
                .get_mut(&ticket.path)
                .is_some_and(|read| read.accept(ticket.comparison.key, answer))
    }

    /// Read lifecycle for ordered body-state projection.
    #[must_use]
    pub const fn status(&self) -> &Reading<GitStatus> {
        &self.status
    }

    /// Summary lifecycle for ordered body-state projection.
    #[must_use]
    pub const fn summary(&self) -> &Reading<DiffSummary> {
        &self.summary
    }

    /// No entry means not requested; it never means unchanged or empty.
    #[must_use]
    pub fn file(&self, path: &RelPath) -> Option<&Reading<FileDiff>> {
        self.files.get(path)
    }

    fn matches(&self, ticket: &ComparisonTicket) -> bool {
        ticket.repository == self.repository && ticket.comparison == self.comparison
    }
}
