//! The pull request's words, chosen from its closed enums (the pattern of
//! `pairing.rs`): a forge's message or a host is a value in a sentence the
//! translator owns, never a sentence of its own.
//!
//! A message with a value can fail to format only when its bundle is
//! broken (the build checks every message and its values); then the
//! generic sentence of the same family stands in, so nothing panics while
//! a frame is drawn.

use std::borrow::Cow;

use vessel_proto::forge::OpenRefusal;

use super::entry::Unavailable;
use super::facts::{CheckCount, ChecksOutcome, Merge, PullPhase, Verdict};
use super::open::OpenProblem;
use crate::localization::{Number, Translator};

/// A message with `host` in it, else `fallback`.
fn with_host(
    formatted: Result<String, crate::localization::FormatError>,
    fallback: &str,
) -> Cow<'_, str> {
    formatted.map_or(Cow::Borrowed(fallback), Cow::Owned)
}

impl Unavailable {
    /// Why "Create PR" cannot run, whole (Paseo's Info toast).
    #[must_use]
    pub fn words<'a>(&self, catalog: &'a Translator) -> Cow<'a, str> {
        let words = catalog.pulls();
        let fallback = words.unavailable();
        match self {
            Self::NoCommits => Cow::Borrowed(words.no_commits()),
            Self::NoBranch => Cow::Borrowed(words.no_branch()),
            Self::NoRemote => Cow::Borrowed(words.no_remote()),
            Self::NotSupported(at) => with_host(words.no_forge(at.as_str()), fallback),
            Self::SignedOut(at) => with_host(words.signed_out(at.as_str()), fallback),
            Self::Refused(at) => with_host(words.refused(at.as_str()), fallback),
            Self::RateLimited(at, _) => with_host(words.rate_limited(at.as_str()), fallback),
            Self::Unreachable(at, _) => with_host(words.unreachable(at.as_str()), fallback),
            Self::Limit => Cow::Borrowed(words.limit()),
            Self::Ended => Cow::Borrowed(words.ended()),
            Self::Unknown => Cow::Borrowed(fallback),
        }
    }
}

impl OpenProblem {
    /// What to change in the form.
    #[must_use]
    pub fn words(self, catalog: &Translator) -> &str {
        let words = catalog.pulls();
        match self {
            Self::TitleBlank => words.title_blank(),
            Self::TitleLong => words.title_long(),
            Self::BodyLong => words.body_long(),
            Self::NotAccepted => words.not_accepted(),
        }
    }
}

/// Why `pr.open` made nothing, as the form's alert title; the push's own
/// refusal and the forge's message are its detail, drawn by the screen.
#[must_use]
pub fn refusal_words<'a>(refusal: &OpenRefusal, catalog: &'a Translator) -> Cow<'a, str> {
    let words = catalog.pulls();
    match refusal {
        OpenRefusal::NotPushed => Cow::Borrowed(words.not_pushed()),
        OpenRefusal::NoCommits => Cow::Borrowed(words.no_commits()),
        OpenRefusal::Push { .. } => Cow::Borrowed(words.push_refused()),
        OpenRefusal::Access { access, .. } => Unavailable::of(access).words(catalog),
        // Before one exists the node names no repository: the forge's
        // own message follows this sentence as the alert's detail.
        OpenRefusal::Rejected { .. } => Cow::Borrowed(words.rejected()),
        OpenRefusal::Unknown => Cow::Borrowed(words.refused_unknown()),
    }
}

impl PullPhase {
    /// The state beside its icon.
    #[must_use]
    pub fn words(self, catalog: &Translator) -> &str {
        let words = catalog.pulls();
        match self {
            Self::Open => words.state_open(),
            Self::Draft => words.state_draft(),
            Self::Merged => words.state_merged(),
            Self::Closed => words.state_closed(),
            Self::Unknown => words.state_unknown(),
        }
    }
}

impl ChecksOutcome {
    /// The checks' headline (Paseo's `checksOverview.headline`).
    #[must_use]
    pub fn words(self, catalog: &Translator) -> &str {
        let words = catalog.pulls();
        match self {
            Self::Failure => words.checks_failure(),
            Self::Pending => words.checks_pending(),
            Self::Success => words.checks_success(),
            Self::None => words.checks_none(),
            Self::Unknown => words.checks_unknown(),
        }
    }
}

impl CheckCount {
    /// One count (Paseo's `checksOverview.count`); empty only with a
    /// broken bundle.
    #[must_use]
    pub fn words(self, catalog: &Translator) -> String {
        let words = catalog.pulls();
        match self {
            Self::Failing(count) => words.count_failing(Number::count(count)),
            Self::Pending(count) => words.count_pending(Number::count(count)),
            Self::Passed(count) => words.count_passed(Number::count(count)),
        }
        .unwrap_or_default()
    }
}

impl Verdict {
    /// The review verdict; `None` when there is none to say.
    #[must_use]
    pub fn words(self, catalog: &Translator) -> Option<&str> {
        let words = catalog.pulls();
        match self {
            Self::Approved => Some(words.approved()),
            Self::ChangesRequested => Some(words.changes_requested()),
            Self::ReviewRequired => Some(words.review_required()),
            Self::None | Self::Unknown => None,
        }
    }
}

impl Merge {
    /// Whether it can merge now.
    #[must_use]
    pub fn words(self, catalog: &Translator) -> &str {
        let words = catalog.pulls();
        match self {
            Self::Ready => words.merge_ready(),
            Self::Conflicts => words.merge_conflicts(),
            Self::Blocked => words.merge_blocked(),
            Self::Unstable => words.merge_unstable(),
            Self::Behind => words.merge_behind(),
            Self::Unknown => words.merge_unknown(),
        }
    }
}

#[cfg(test)]
#[path = "words_tests.rs"]
mod tests;
