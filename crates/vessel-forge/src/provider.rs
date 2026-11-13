//! The one trait every forge implements (ADR 0029), and the
//! provider-neutral values it answers with.
//!
//! [Decision Log]
//! - Purpose: the node watches, opens and merges pull requests without
//!   knowing which forge a repository is on; GitHub is built now, GitLab
//!   and Gitea later.
//! - Constraint: what the three references abstract (agent-orchestrator's
//!   read and write ports, GOAT's provider and pull request API, Paseo's
//!   forge service) differs where the forges do: GitLab's merge request
//!   `iid`, its pipelines instead of checks, approvals instead of "changes
//!   requested", no pinned head for rebase; Gitea's missing pinned head.
//!   The trait names those once ([`Forge::pins_head`], the neutral
//!   [`Discussion`]) so a later provider fills the same values.
//! - Choice: an object-safe trait with boxed futures, held as `Arc<dyn
//!   Forge>`: an open extension point (one provider per host, chosen at
//!   run time from a remote's URL, and the tests' double, `fake`), which
//!   AGENTS.md section 2 allows `dyn` for. Every call says whether a
//!   person waits on it ([`Priority`]), so a provider can keep slots and
//!   rate budget for people.
//! - Conditional reads are the provider's: [`Forge::changed`] answers
//!   whether anything moved since its last look, so a forge without
//!   conditional requests can answer `true` and the node reads whole.

use std::fmt;
use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};
use vessel_proto::forge::{
    CheckName, ForgeHost, ForgeKind, ForgeLogin, ForgeUrl, GhState, MergeMethod, PullBody,
    PullNumber, PullRequest, PullState, PullTitle, RefusedReason, RepoRef,
};
use vessel_proto::workspace::{BranchName, CommitId};

/// A boxed future, as the trait's methods answer.
pub type Answer<'a, T> = Pin<Box<dyn Future<Output = Result<T, ForgeError>> + Send + 'a>>;

/// Who waits on a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    /// A person's call: first in line, and not held back by the
    /// background's rate budget.
    Person,
    /// The watcher, or an agent's refresh.
    Background,
}

/// What a conditional read found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fresh<T> {
    /// Something changed; here it is.
    Changed(T),
    /// Nothing changed since the last look.
    Same,
}

/// A pull request found for a branch, before it is read whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// Its number.
    pub number: PullNumber,
    /// Its state.
    pub state: PullState,
    /// Its head commit.
    pub head: CommitId,
    /// When it last changed.
    pub updated_at: DateTime<Utc>,
}

/// An identifier a forge gives a review, a thread or a comment: opaque,
/// compared and recorded only (in feedback signatures).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemId(String);

impl ItemId {
    /// An id from a forge's text, `None` when it is not one short line of
    /// printable characters (it goes into a stored signature).
    #[must_use]
    pub fn new(text: &str) -> Option<Self> {
        let ok = !text.is_empty()
            && text.len() <= 128
            && text.bytes().all(|byte| byte.is_ascii_graphic());
        ok.then(|| Self(text.to_owned()))
    }

    /// The id as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Who wrote a review or a comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Author {
    /// Their login, when the forge named a valid one.
    pub login: Option<ForgeLogin>,
    /// A bot, not a person.
    pub bot: bool,
}

/// What a review said.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewVerdict {
    /// It approved.
    Approved,
    /// It asked for changes.
    ChangesRequested,
    /// It commented, or anything else.
    Other,
}

/// A review on the pull request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Review {
    /// Its id.
    pub id: ItemId,
    /// Who.
    pub author: Author,
    /// What it said.
    pub verdict: ReviewVerdict,
    /// Its text, as the forge has it (quoted only under the feedback's
    /// rules).
    pub body: String,
    /// Its page.
    pub url: Option<ForgeUrl>,
}

/// A comment of a review thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    /// Its id.
    pub id: ItemId,
    /// Who.
    pub author: Author,
    /// Its text.
    pub body: String,
    /// Its page.
    pub url: Option<ForgeUrl>,
}

/// A review thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thread {
    /// Its id.
    pub id: ItemId,
    /// Resolved.
    pub resolved: bool,
    /// The file it is on, as the forge names it.
    pub path: Option<String>,
    /// The line, when on one.
    pub line: Option<u32>,
    /// Its comments, oldest first.
    pub comments: Vec<Comment>,
}

/// A failing check, for feedback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedCheck {
    /// Its name.
    pub name: CheckName,
    /// Its page.
    pub url: Option<ForgeUrl>,
}

/// What feedback reads beside the pull request's facts: its reviews,
/// threads and failing checks, the newest first, bounded by the provider.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Discussion {
    /// The latest review per person, the current head's.
    pub reviews: Vec<Review>,
    /// Review threads.
    pub threads: Vec<Thread>,
    /// Failing checks on the head.
    pub failed: Vec<FailedCheck>,
}

/// A pull request read whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Read {
    /// Its facts, as the wire carries them.
    pub pull: PullRequest,
    /// What feedback needs.
    pub discussion: Discussion,
}

/// What to open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opening {
    /// The branch to merge.
    pub head: BranchName,
    /// The branch to merge into.
    pub base: BranchName,
    /// The title.
    pub title: PullTitle,
    /// The description.
    pub body: PullBody,
    /// A draft.
    pub draft: bool,
}

/// What opening did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Opened {
    /// Made: its number.
    Created(PullNumber),
    /// One exists for that head already.
    Exists,
    /// Nothing between the base and the head.
    NoCommits,
    /// Refused, in the forge's words.
    Rejected(String),
}

/// What merging did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Merged {
    /// Merged: the commit it made on the base.
    Done(CommitId),
    /// The head is not the one given.
    HeadMoved,
    /// The forge says it cannot merge now.
    NotMergeable(Option<String>),
    /// That method is not allowed.
    MethodNotAllowed,
    /// The repository requires its merge queue.
    MergeQueue,
    /// Refused, in the forge's words.
    Rejected(String),
}

/// Why a request to a forge failed.
#[derive(Debug, thiserror::Error)]
pub enum ForgeError {
    /// No token for the host.
    #[error("not signed in to the forge")]
    SignedOut {
        /// What `gh` said.
        gh: GhState,
    },
    /// The forge refused the token.
    #[error("the forge refused the token")]
    Refused {
        /// Why.
        reason: RefusedReason,
    },
    /// Its rate limit holds requests until then.
    #[error("the forge's rate limit holds requests until {until}")]
    RateLimited {
        /// When.
        until: DateTime<Utc>,
    },
    /// It could not be reached, or did not answer in time.
    #[error("the forge could not be reached")]
    Unreachable(#[source] reqwest::Error),
    /// It answered nothing usable: a closed connection, or the test
    /// double told to fail.
    #[error("the forge did not answer")]
    Down,
    /// It answered with a server error.
    #[error("the forge answered {status}")]
    Server {
        /// The status.
        status: u16,
    },
    /// No such pull request.
    #[error("no such pull request")]
    NotFound,
    /// Its answer was not what this provider reads.
    #[error("the forge's answer could not be read ({what})")]
    Unexpected {
        /// What was being read.
        what: &'static str,
    },
    /// Too many requests wait already.
    #[error("too many forge requests wait already")]
    Busy,
}

/// A forge: one per host (ADR 0029).
pub trait Forge: Send + Sync + fmt::Debug {
    /// Its kind.
    fn kind(&self) -> ForgeKind;

    /// Whether it can merge with `method` only at a given head (GitHub:
    /// all three; GitLab not rebase; Gitea none).
    fn pins_head(&self, method: MergeMethod) -> bool;

    /// Who the token for `host` is.
    fn whoami<'a>(&'a self, host: &'a ForgeHost) -> Answer<'a, Option<ForgeLogin>>;

    /// The pull requests whose head is `branch` in `repo`, the most
    /// recently updated first (at most 10); [`Fresh::Same`] when nothing
    /// changed since the last look, which a [`Priority::Person`] request
    /// never answers (it is not conditional).
    fn find<'a>(
        &'a self,
        repo: &'a RepoRef,
        branch: &'a BranchName,
        priority: Priority,
    ) -> Answer<'a, Fresh<Vec<Found>>>;

    /// Whether `number` (its head at `head`) changed since the last look:
    /// itself, its checks or its statuses; always for a
    /// [`Priority::Person`] request (it is not conditional).
    fn changed<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: PullNumber,
        head: &'a CommitId,
        priority: Priority,
    ) -> Answer<'a, bool>;

    /// `number`, read whole.
    fn read<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: PullNumber,
        priority: Priority,
    ) -> Answer<'a, Read>;

    /// Open a pull request (a person's call).
    fn open<'a>(&'a self, repo: &'a RepoRef, opening: &'a Opening) -> Answer<'a, Opened>;

    /// Merge `number` with `method` at `head` (a person's call).
    fn merge<'a>(
        &'a self,
        repo: &'a RepoRef,
        number: PullNumber,
        method: MergeMethod,
        head: &'a CommitId,
    ) -> Answer<'a, Merged>;

    /// Whether `login` may write to `repo` (feedback quotes only those).
    fn can_write<'a>(&'a self, repo: &'a RepoRef, login: &'a ForgeLogin) -> Answer<'a, bool>;
}
