//! Read identity for a pane: transport replies may outlive repository, mode,
//! base and whitespace changes. Reuse the accepted monotonic request issuer.

use super::{
    Body, Comparison, Demand, DiffRead, PanelRead, ReadFailure, RepositoryKey, RepositoryRead, body,
};
use crate::workspace::git::RequestKey;
use std::collections::HashMap;
use vessel_proto::workspace::{DiffSummary, FileDiff, GitReadReason, GitStatus, RelPath};

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
    reason: GitReadReason,
    key: RequestKey,
}

impl ComparisonTicket {
    /// The link routes only to this actual node/root.
    #[must_use]
    pub const fn repository(&self) -> RepositoryKey {
        self.repository
    }

    /// A person refresh bypasses cache freshness without changing identity.
    #[must_use]
    pub const fn reason(&self) -> GitReadReason {
        self.reason
    }

    /// A legacy node must not substitute All for unsupported Committed.
    #[must_use]
    pub const fn comparison(&self) -> &Comparison {
        &self.comparison
    }
}

/// Source generation for retained diff cells. A visual row index alone must
/// never authorize a comment after another file read replaced the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FileStamp {
    repository: RepositoryKey,
    key: RequestKey,
}

/// File body identity also includes its relative path; counts are not a version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTicket {
    comparison: ComparisonTicket,
    path: RelPath,
}

impl FileTicket {
    pub(super) const fn stamp(&self) -> FileStamp {
        FileStamp {
            repository: self.comparison.repository,
            key: self.comparison.key,
        }
    }

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
    status: PanelRead<GitStatus>,
    summary: PanelRead<DiffSummary>,
    files: HashMap<RelPath, PanelRead<FileDiff>>,
}

impl PanelReads {
    /// A new target has no usable facts from the prior repository.
    #[must_use]
    pub fn new(repository: RepositoryKey, comparison: Comparison) -> Self {
        Self {
            repository,
            comparison,
            status: PanelRead::Empty,
            summary: PanelRead::Empty,
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

    /// Retiring a selected connection settles loading at once. Replies from
    /// its old epoch cannot restore facts, even after returning to this node.
    pub fn retire(&mut self) {
        self.status = PanelRead::Failed(ReadFailure::DestinationChanged);
        self.summary = PanelRead::Failed(ReadFailure::DestinationChanged);
        self.files.clear();
    }

    /// Selecting another mode/base/filter invalidates dependent reads at once.
    pub fn select(&mut self, comparison: Comparison) {
        if self.comparison != comparison {
            self.comparison = comparison;
            self.summary = PanelRead::Empty;
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
        answer: Result<GitStatus, ReadFailure>,
    ) -> bool {
        ticket.repository == self.repository && self.status.accept(ticket.key, answer)
    }

    /// Begin a read of exactly the selected comparison.
    #[must_use]
    pub fn begin_summary(&mut self, key: RequestKey) -> ComparisonTicket {
        self.begin_summary_for(key, GitReadReason::View)
    }

    /// Force is explicit and typed, never inferred from a repaint or retry.
    #[must_use]
    pub fn begin_summary_for(
        &mut self,
        key: RequestKey,
        reason: GitReadReason,
    ) -> ComparisonTicket {
        self.summary.start(key);
        ComparisonTicket {
            repository: self.repository,
            comparison: self.comparison.clone(),
            reason,
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
        answer: Result<DiffSummary, ReadFailure>,
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
                reason: GitReadReason::View,
                key,
            },
            path,
        }
    }

    /// The path and echoed identity must both match, including on send failure.
    #[must_use]
    pub fn finish_file(
        &mut self,
        ticket: &FileTicket,
        answer: Result<FileDiff, ReadFailure>,
    ) -> bool {
        if !self.matches(&ticket.comparison) {
            return false;
        }
        let answer = answer.and_then(|file| {
            if file.path == ticket.path {
                Ok(file)
            } else {
                Err(ReadFailure::UnexpectedFile {
                    requested: ticket.path.clone(),
                    received: file.path,
                })
            }
        });
        self.files
            .get_mut(&ticket.path)
            .is_some_and(|read| read.accept(ticket.comparison.key, answer))
    }

    /// Read lifecycle for ordered body-state projection.
    #[must_use]
    pub const fn status(&self) -> &PanelRead<GitStatus> {
        &self.status
    }

    /// Summary lifecycle for ordered body-state projection.
    #[must_use]
    pub const fn summary(&self) -> &PanelRead<DiffSummary> {
        &self.summary
    }

    /// One failure/loading/empty precedence for every consumer. A retained
    /// refresh is shown, but a failed status always hides an older diff.
    #[must_use]
    pub fn body(&self, demand: Demand) -> Body<'_> {
        let repository = match &self.status {
            PanelRead::Failed(why) => RepositoryRead::Failed(why),
            PanelRead::Empty | PanelRead::Loading { previous: None, .. } => RepositoryRead::Waiting,
            PanelRead::Ready(status)
            | PanelRead::Loading {
                previous: Some(status),
                ..
            } => RepositoryRead::Ready(status),
        };
        let diff = match &self.summary {
            PanelRead::Failed(ReadFailure::TooLarge) => DiffRead::TooLarge,
            PanelRead::Failed(why) => DiffRead::Failed(why),
            PanelRead::Empty | PanelRead::Loading { previous: None, .. } => DiffRead::Waiting,
            PanelRead::Ready(summary)
            | PanelRead::Loading {
                previous: Some(summary),
                ..
            } => DiffRead::Ready(summary),
        };
        body(demand, repository, diff, &self.comparison)
    }

    /// No entry means not requested; it never means unchanged or empty.
    #[must_use]
    pub fn file(&self, path: &RelPath) -> Option<&PanelRead<FileDiff>> {
        self.files.get(path)
    }

    fn matches(&self, ticket: &ComparisonTicket) -> bool {
        ticket.repository == self.repository && ticket.comparison == self.comparison
    }
}
