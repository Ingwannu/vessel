//! Read identity for a pane: transport replies may outlive repository, mode,
//! base and whitespace changes. Reuse the accepted monotonic request issuer.
//!
//! [Decision Log]
//! - A retained summary may stay drawn while refreshing, but equal paths/counts
//!   cannot make an old row callback current. Ready data alone lost its key.
//! - Keep the accepted summary ticket in this existing read store; refresh,
//!   mode changes and retirement invalidate activation before new work begins.
//! - File callbacks capture that opaque generation and recheck it synchronously.
//!   No second key book, resolver or content-based authority is introduced.
//! - This only correlates local reads; original connection/pane admission and
//!   filesystem snapshot guarantees remain their respective owners' concern.

use super::{
    Body, Comparison, Demand, DiffRead, FilePages, PanelRead, ReadFailure, RepositoryKey,
    RepositoryRead, body,
};
use crate::workspace::git::RequestKey;
use std::collections::HashMap;
use vessel_proto::workspace::{
    DiffSummary, FileDiff, GitHunkOffset, GitReadReason, GitStatus, RelPath,
};

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
    summary_key: Option<RequestKey>,
    from_hunk: GitHunkOffset,
}

impl FileTicket {
    pub(super) const fn request_key(&self) -> RequestKey {
        self.comparison.key
    }

    /// Requested continuation, echoed intact even for a rejected send.
    #[must_use]
    pub const fn from_hunk(&self) -> GitHunkOffset {
        self.from_hunk
    }

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
    accepted_summary: Option<ComparisonTicket>,
    files: HashMap<RelPath, PanelRead<FilePages>>,
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
            accepted_summary: None,
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
        self.accepted_summary = None;
        self.files.clear();
    }

    /// Selecting another mode/base/filter invalidates dependent reads at once.
    pub fn select(&mut self, comparison: Comparison) {
        if self.comparison != comparison {
            self.comparison = comparison;
            self.summary = PanelRead::Empty;
            self.accepted_summary = None;
            self.files.clear();
        }
    }

    /// Loading retains display facts but does not authorize writes from them.
    #[must_use]
    pub fn begin_status(&mut self, key: RequestKey) -> StatusTicket {
        self.accepted_summary = None;
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
        self.accepted_summary = None;
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
        self.accepted_summary = self.summary.ready().map(|_| ticket.clone());
        self.files.clear();
        true
    }

    /// Low-level explicit body reads also serve immutable/direct-file callers.
    /// Drawn-row callbacks use `begin_captured_file` so a stale summary cannot
    /// start another request. Issuing again supersedes this file's old read.
    #[must_use]
    pub fn begin_file(&mut self, path: RelPath, key: RequestKey) -> FileTicket {
        self.begin_file_at(path, key, GitHunkOffset::default())
    }

    fn begin_file_at(
        &mut self,
        path: RelPath,
        key: RequestKey,
        from_hunk: GitHunkOffset,
    ) -> FileTicket {
        self.files.entry(path.clone()).or_default().start(key);
        FileTicket {
            comparison: ComparisonTicket {
                repository: self.repository,
                comparison: self.comparison.clone(),
                reason: GitReadReason::View,
                key,
            },
            path,
            summary_key: self.accepted_summary.as_ref().map(|ticket| ticket.key),
            from_hunk,
        }
    }

    /// The path and echoed identity must both match, including on send failure.
    #[must_use]
    pub fn finish_file(
        &mut self,
        ticket: &FileTicket,
        answer: Result<FileDiff, ReadFailure>,
    ) -> bool {
        if !self.matches(&ticket.comparison)
            || ticket.summary_key != self.accepted_summary.as_ref().map(|ticket| ticket.key)
        {
            return false;
        }
        let Some(read) = self.files.get_mut(&ticket.path) else {
            return false;
        };
        if read.outstanding() != Some(ticket.comparison.key) {
            return false;
        }
        let answer = answer.and_then(|file| {
            if file.path == ticket.path {
                FilePages::accept(read.shown(), ticket, file)
            } else {
                Err(ReadFailure::UnexpectedFile {
                    requested: ticket.path.clone(),
                    received: file.path,
                })
            }
        });
        read.accept(ticket.comparison.key, answer)
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
    pub fn file(&self, path: &RelPath) -> Option<&PanelRead<FilePages>> {
        self.files.get(path)
    }

    /// Capture only a file in the actually accepted summary, never retained
    /// refresh data. The generation is local read correlation, not authority
    /// or a promise of an atomic filesystem snapshot across paged reads.
    #[must_use]
    pub fn file_target(&self, path: &RelPath) -> Option<FileReadTarget> {
        if self.status.ready().is_none() {
            return None;
        }
        let summary = self.summary.ready()?;
        let ticket = self.accepted_summary.as_ref()?;
        if !summary.files.iter().any(|file| file.path == *path) {
            return None;
        }
        Some(FileReadTarget {
            comparison: ticket.clone(),
            path: path.clone(),
            previous: None,
            from_hunk: GitHunkOffset::default(),
        })
    }

    /// Only a Ready body's exact receipt offers its next page. A queued
    /// continuation cannot restart after another body with the same path.
    #[must_use]
    pub fn next_file_target(&self, path: &RelPath) -> Option<FileReadTarget> {
        let mut target = self.file_target(path)?;
        let previous = self.files.get(path)?.ready()?;
        target.previous = Some(previous.key());
        target.from_hunk = GitHunkOffset::new(previous.file().more_from?);
        Some(target)
    }

    /// A deferred row callback must recheck this immediately before issuing a
    /// key or a request. Equal paths/counts and comparison A-to-B-to-A do not
    /// make a callback from a retired summary current again.
    #[must_use]
    pub fn file_target_is_current(&self, target: &FileReadTarget) -> bool {
        let current = if target.previous.is_some() {
            self.next_file_target(&target.path)
        } else {
            self.file_target(&target.path)
        };
        current.as_ref() == Some(target)
    }

    /// Start only the exact captured generation. This rechecks even if the
    /// caller already tested currentness before allocating its lifetime key.
    #[must_use]
    pub fn begin_captured_file(
        &mut self,
        target: &FileReadTarget,
        key: RequestKey,
    ) -> Option<FileTicket> {
        if !self.file_target_is_current(target) {
            return None;
        }
        // Repainting the same first-page row is not an explicit refresh.
        // Failed pages may retry; ready/loading data must not be restarted by
        // a second queued paint callback. Direct reads retain begin_file.
        if target.previous.is_none()
            && self
                .files
                .get(&target.path)
                .is_some_and(|read| read.ready().is_some() || read.outstanding().is_some())
        {
            return None;
        }
        Some(self.begin_file_at(target.path.clone(), key, target.from_hunk))
    }

    fn matches(&self, ticket: &ComparisonTicket) -> bool {
        ticket.repository == self.repository && ticket.comparison == self.comparison
    }
}

/// Opaque activation from an accepted summary's exact generation.
/// The owner must additionally retain and check its original connection,
/// pane incarnation and visibility; this value grants no write capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReadTarget {
    comparison: ComparisonTicket,
    path: RelPath,
    previous: Option<RequestKey>,
    from_hunk: GitHunkOffset,
}
