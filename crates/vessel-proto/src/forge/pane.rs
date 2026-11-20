//! What the pull request pane reads beside `pr.get` (ADR 0088, parts 2 to
//! 4): every check with its run and timing (on `pr.get`'s `Checks`), the
//! activity timeline (`pr.timeline`) and one check's details and failed
//! jobs' log tails (`pr.check`). Paseo's `pull_request_timeline_*` and
//! `checkout.forge.get_check_details.*`.
//!
//! [Decision Log]
//! - Purpose: Paseo's pane draws every check grouped by outcome, the
//!   reviews, comments and review threads in time order, and a failed
//!   check's logs for "Add logs to chat"; the node sent none of these.
//! - Constraint: a pull request is reached through its session (ADR 0029),
//!   so a check is named by the run id the node's own last read gave,
//!   never by a URL; every value a forge supplies is made a bounded wire
//!   value (`names`), and an answer stays far under a 16 MiB frame (the
//!   provider reads at most 4 MiB of any one answer).
//! - Choice: two reads with Paseo's bounds; times on the wire, not
//!   Paseo's preformatted durations (a client says "running 2m" from
//!   them, and a pending check's time goes on growing). `access` says, as
//!   on `pr.get`, why the node could not read.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

use super::access::ForgeAccess;
use super::names::{
    CheckName, ForgeItemId, ForgeLogin, ForgePath, ForgeUrl, InvalidForgeValue, PullBody,
    PullNumber, PullTitle,
};
use super::pull::CheckState;
use crate::ProtocolVersion;
use crate::id::SessionId;

/// `pr.timeline` and `pr.check`, and the checks' runs and times on
/// `pr.get` (ADR 0088). Drafted as 39, after the shared terminals' 37 and
/// the 38 drafted before it; the integrator assigns it in merge order.
pub const PULL_REQUEST_PANE_PROTOCOL: Option<ProtocolVersion> = Some(ProtocolVersion::new(39));

/// A check run's, a workflow run's or a job's number on the forge: 1 to
/// 2^53 − 1, what a page holds exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct RunId(u64);

impl RunId {
    /// The largest id a page holds exactly.
    pub const MAX: u64 = (1 << 53) - 1;

    /// `id`, when it is 1 to [`Self::MAX`].
    ///
    /// # Errors
    /// Zero, or past [`Self::MAX`].
    pub const fn new(id: u64) -> Result<Self, InvalidForgeValue> {
        if id == 0 || id > Self::MAX {
            Err(InvalidForgeValue {
                kind: "forge run id",
                rule: "1 to 2^53 - 1",
            })
        } else {
            Ok(Self(id))
        }
    }

    /// The number.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for RunId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(u64::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// `pr.timeline`: a session's pull request's reviews, comments and review
/// threads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetPullTimeline {
    /// The session.
    pub session: SessionId,
}

/// What `pr.timeline` answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PullTimeline {
    /// `ready` when the node read the forge, else why not (then no items).
    pub access: ForgeAccess,
    /// The pull request the items are of.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<PullNumber>,
    /// Oldest first, then by id (Paseo's `compareTimelineItems`).
    pub items: Vec<TimelineItem>,
    /// A kind had more than one read takes (100 each, 100 comments a
    /// thread).
    pub truncated: bool,
}

impl PullTimeline {
    /// The most reviews, conversation comments and threads read (each).
    pub const MAX_EACH: usize = 100;
    /// The most comments read of one thread.
    pub const MAX_THREAD_COMMENTS: usize = 100;
}

/// What a review said, as Paseo's pane groups it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ReviewState {
    /// It approved.
    Approved,
    /// It asked for changes.
    ChangesRequested,
    /// It commented.
    Commented,
    /// A verdict a newer node knows and this client does not.
    #[serde(other)]
    Unknown,
}

/// Where a review thread's comment is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThreadPlace {
    /// The thread.
    pub thread: ForgeItemId,
    /// The file, as the forge names it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<ForgePath>,
    /// The (last) line, when on one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// The first line of a range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
    /// The thread was resolved.
    pub resolved: bool,
    /// The lines changed since.
    pub outdated: bool,
}

/// One entry of the timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimelineItem {
    /// A review.
    Review {
        /// Its id.
        id: ForgeItemId,
        /// Who, when the forge named a valid login.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        author: Option<ForgeLogin>,
        /// Its text (markdown, as the forge has it).
        body: PullBody,
        /// When it was submitted.
        created_at: DateTime<Utc>,
        /// Its page.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<ForgeUrl>,
        /// What it said.
        state: ReviewState,
    },
    /// A conversation comment, or one comment of a review thread.
    Comment {
        /// Its id.
        id: ForgeItemId,
        /// Who.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        author: Option<ForgeLogin>,
        /// Its text.
        body: PullBody,
        /// When.
        created_at: DateTime<Utc>,
        /// Its page.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<ForgeUrl>,
        /// The review it was written in.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        review: Option<ForgeItemId>,
        /// Its thread, when it is a review thread's.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        thread: Option<ThreadPlace>,
    },
}

impl TimelineItem {
    /// When it was written.
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        match self {
            Self::Review { created_at, .. } | Self::Comment { created_at, .. } => *created_at,
        }
    }

    /// Its id.
    #[must_use]
    pub const fn id(&self) -> &ForgeItemId {
        match self {
            Self::Review { id, .. } | Self::Comment { id, .. } => id,
        }
    }
}

/// `pr.check`: one check of a session's pull request, with its failed
/// jobs' logs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetCheckDetails {
    /// The session.
    pub session: SessionId,
    /// The check run, as `pr.get` named it.
    pub run: RunId,
}

/// What `pr.check` answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CheckDetails {
    /// `ready` when the node read the forge, else why not.
    pub access: ForgeAccess,
    /// The check; `None` when not read, or when `run` is not one of the
    /// pull request's head's checks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<Box<CheckReport>>,
}

/// One check run read whole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CheckReport {
    /// The check run.
    pub run: RunId,
    /// Its name.
    pub name: CheckName,
    /// Its result.
    pub state: CheckState,
    /// Its page on the forge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<ForgeUrl>,
    /// The page its app links to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details_url: Option<ForgeUrl>,
    /// What it reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<CheckOutput>,
    /// Its annotations, at most [`CheckReport::MAX_ANNOTATIONS`].
    pub annotations: Vec<Annotation>,
    /// Its workflow run's failed jobs, at most [`CheckReport::MAX_FAILED_JOBS`].
    pub failed_jobs: Vec<FailedJob>,
    /// More annotations or jobs were reported than were read.
    pub truncated: bool,
}

impl CheckReport {
    /// The most annotations read (Paseo's one page of
    /// `CHECK_ANNOTATION_PAGE_MAX`).
    pub const MAX_ANNOTATIONS: usize = 20;
    /// The most failed jobs read (Paseo's `FAILED_CHECK_JOB_LIMIT`).
    pub const MAX_FAILED_JOBS: usize = 5;
}

/// A check run's output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CheckOutput {
    /// Its title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<PullTitle>,
    /// Its summary (markdown).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<PullBody>,
    /// Its details (markdown).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<PullBody>,
}

/// How serious an annotation is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum AnnotationLevel {
    /// A note.
    Notice,
    /// A warning.
    Warning,
    /// A failure.
    Failure,
    /// A level a newer node knows and this client does not.
    #[serde(other)]
    Unknown,
}

/// One annotation of a check run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Annotation {
    /// The file.
    pub path: ForgePath,
    /// Its first line.
    pub start_line: u32,
    /// Its last line.
    pub end_line: u32,
    /// How serious.
    pub level: AnnotationLevel,
    /// What it says.
    pub message: PullBody,
    /// Its title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<PullTitle>,
}

/// A failed job of the check's workflow run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FailedJob {
    /// The job.
    pub job: RunId,
    /// Its name.
    pub name: CheckName,
    /// Its result.
    pub state: CheckState,
    /// Its page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<ForgeUrl>,
    /// When it ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// The end of its log, when it could be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log: Option<LogTail>,
}

/// The end of a job's log: at most [`LogTail::MAX_LINES`] lines and
/// [`LogTail::MAX_BYTES`] bytes, cut at a character and a line boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LogTail {
    /// The lines (control characters other than tab and newline removed).
    pub text: PullBody,
    /// Earlier lines were left out.
    pub cut: bool,
}

impl LogTail {
    /// The most lines kept (Paseo's `CHECK_LOG_TAIL_MAX_LINES`).
    pub const MAX_LINES: usize = 200;
    /// The most bytes kept (Paseo's `CHECK_LOG_TAIL_MAX_BYTES`).
    pub const MAX_BYTES: usize = 16 * 1024;

    /// The tail of `log`, a job's whole or partial log as read; `cut`
    /// when the read itself already left the start out.
    #[must_use]
    pub fn of(log: &[u8], cut: bool) -> Self {
        let text = String::from_utf8_lossy(log);
        let clean: String = text
            .chars()
            .filter(|character| !character.is_control() || matches!(character, '\n' | '\t'))
            .collect();
        let trimmed = clean.trim_end_matches('\n');
        let mut start = trimmed.len().saturating_sub(Self::MAX_BYTES);
        while !trimmed.is_char_boundary(start) {
            start = start.saturating_add(1);
        }
        let mut cut = cut || start > 0;
        let mut tail = trimmed.get(start..).unwrap_or_default();
        if start > 0
            && let Some((_partial, rest)) = tail.split_once('\n')
        {
            tail = rest;
        }
        let lines: Vec<&str> = tail.split('\n').collect();
        if lines.len() > Self::MAX_LINES {
            cut = true;
        }
        let kept = lines
            .get(lines.len().saturating_sub(Self::MAX_LINES)..)
            .unwrap_or_default()
            .join("\n");
        Self {
            text: PullBody::lossy(&kept),
            cut,
        }
    }
}

#[cfg(test)]
mod tests;
