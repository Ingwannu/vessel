//! What "Add to chat" hands the composer from the pull request pane:
//! Paseo's `pull-request-panel/context-attachment.ts` (pinned 0.15.4),
//! the same headings and lines, for a comment, a review, a thread and a
//! failed check with its logs (ADR 0088 parts 3 and 4).
//!
//! [Decision Log]
//! - Purpose: the agent reads what a reviewer said, or why a check
//!   failed, without the person copying it.
//! - Constraint: the text goes to the agent, not to the person; Paseo
//!   writes it in English with its own ages ("3h ago").
//! - Choice: Paseo's text line for line, GitHub only; ages in Paseo's
//!   English whatever the screen's language.

use chrono::{DateTime, Utc};
use vessel_proto::forge::{
    AnnotationLevel, CheckReport, CheckRun, CheckState, ReviewState, ThreadPlace, TimelineItem,
};

use super::timeline::{Age, Thread, thread_path};

/// Which tray item an attachment is (Paseo's `forge.change_request_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachKind {
    /// A comment, or a whole thread.
    Comment,
    /// A review.
    Review,
    /// A check and its logs.
    Check,
}

/// One attachment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    /// Its tray kind.
    pub kind: AttachKind,
    /// Its identity in the tray (a second add replaces the first).
    pub id: String,
    /// Its title: the author, the thread's place, or the check.
    pub title: String,
    /// The text the agent reads.
    pub text: String,
    /// Its page, when there is one.
    pub url: Option<String>,
}

/// The pull request an attachment is about.
#[derive(Debug, Clone, Copy)]
pub struct About<'a> {
    /// Its number.
    pub number: u64,
    /// Its title.
    pub title: &'a str,
    /// Its page.
    pub url: &'a str,
    /// When the attachment is made, for ages.
    pub now: DateTime<Utc>,
}

impl About<'_> {
    /// Paseo's two lines after the heading.
    fn lines(&self, heading: &str) -> Vec<String> {
        vec![
            heading.to_owned(),
            format!("Pull request: #{} {}", self.number, self.title),
            format!("Pull request URL: {}", self.url),
        ]
    }
}

impl Age {
    /// Paseo's `formatAge`, in its English.
    #[must_use]
    pub fn english(self) -> String {
        match self {
            Self::JustNow => "just now".to_owned(),
            Self::Minutes(count) => format!("{count}m ago"),
            Self::Hours(count) => format!("{count}h ago"),
            Self::Days(count) => format!("{count}d ago"),
            Self::Months(count) => format!("{count}mo ago"),
            Self::Years(count) => format!("{count}y ago"),
        }
    }
}

/// An item's author, its words and its page.
fn parts(item: &TimelineItem) -> (String, &str, String) {
    let (author, body, url) = match item {
        TimelineItem::Review {
            author, body, url, ..
        }
        | TimelineItem::Comment {
            author, body, url, ..
        } => (author, body, url),
    };
    (
        author
            .as_ref()
            .map_or_else(|| "unknown".to_owned(), |login| login.as_str().to_owned()),
        body.as_str().trim(),
        url.as_ref()
            .map(|url| url.as_str().to_owned())
            .unwrap_or_default(),
    )
}

/// Paseo's `canAddPullRequestActivityToChat`: words, or a change request.
#[must_use]
pub fn can_add(item: &TimelineItem) -> bool {
    let (_, body, _) = parts(item);
    !body.is_empty()
        || matches!(
            item,
            TimelineItem::Review {
                state: ReviewState::ChangesRequested,
                ..
            }
        )
}

/// A review state as Paseo's wire names it.
const fn review_state(state: ReviewState) -> &'static str {
    match state {
        ReviewState::Approved => "approved",
        ReviewState::ChangesRequested => "changes_requested",
        ReviewState::Commented | ReviewState::Unknown => "commented",
    }
}

/// Paseo's `formatPullRequestActivityLocation`: the place, then resolved
/// or not, outdated or current, and a short thread id.
fn location(place: &ThreadPlace) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.extend(thread_path(place));
    parts.push(if place.resolved { "resolved" } else { "unresolved" }.to_owned());
    parts.push(if place.outdated { "outdated" } else { "current" }.to_owned());
    let thread = place.thread.as_str();
    if thread.chars().count() <= 24 {
        parts.push(format!("thread {thread}"));
    }
    parts.join(" · ")
}

/// A comment or a review (Paseo's activity text); `None` for one with
/// nothing to add.
#[must_use]
pub fn activity(about: &About<'_>, item: &TimelineItem) -> Option<Attachment> {
    if !can_add(item) {
        return None;
    }
    let (author, body, url) = parts(item);
    let (kind, heading, state, place) = match item {
        TimelineItem::Comment { thread, .. } => (
            AttachKind::Comment,
            "GitHub pull request comment",
            None,
            thread.as_ref(),
        ),
        TimelineItem::Review { state, .. } => (
            AttachKind::Review,
            "GitHub pull request review",
            Some(review_state(*state)),
            None,
        ),
    };
    let mut lines = about.lines(heading);
    lines.push(format!("URL: {url}"));
    lines.push(format!("Author: {author}"));
    if let Some(state) = state {
        lines.push(format!("State: {state}"));
    }
    lines.push(format!(
        "Created: {}",
        Age::of(item.created_at(), about.now).english()
    ));
    if let Some(place) = place {
        lines.push(format!("Location: {}", location(place)));
    }
    if !body.is_empty() {
        lines.push(String::new());
        lines.push(body.to_owned());
    }
    Some(Attachment {
        kind,
        id: format!("{}:{}", about.number, item.id().as_str()),
        title: author,
        text: lines.join("\n"),
        url: (!url.is_empty()).then_some(url),
    })
}

/// A whole thread, its comments with words in order; `None` when none
/// has any.
#[must_use]
pub fn thread(about: &About<'_>, thread: &Thread<'_>) -> Option<Attachment> {
    let root = thread.comments.first()?;
    let said: Vec<String> = thread
        .comments
        .iter()
        .filter_map(|item| {
            let (author, body, _) = parts(item);
            (!body.is_empty()).then(|| {
                let age = Age::of(item.created_at(), about.now).english();
                format!("{author} ({age}):\n{body}")
            })
        })
        .collect();
    if said.is_empty() {
        return None;
    }
    let (_, _, url) = parts(root);
    let path = thread_path(thread.place);
    let mut lines = about.lines("GitHub pull request review thread");
    lines.push(format!("URL: {url}"));
    if let Some(path) = &path {
        lines.push(format!("Location: {path}"));
    }
    lines.push(format!(
        "Thread state: {}",
        if thread.place.resolved {
            "resolved"
        } else {
            "unresolved"
        }
    ));
    if thread.place.outdated {
        lines.push("Note: this thread is outdated (the code it refers to has changed)".to_owned());
    }
    lines.push(String::new());
    lines.push(said.join("\n\n---\n\n"));
    Some(Attachment {
        kind: AttachKind::Comment,
        id: format!("{}:{}", about.number, thread.key()),
        title: path.unwrap_or_else(|| "Discussion thread".to_owned()),
        text: lines.join("\n"),
        url: (!url.is_empty()).then_some(url),
    })
}

/// A check state as Paseo's wire names it.
const fn check_status(state: CheckState) -> &'static str {
    match state {
        CheckState::Passed => "success",
        CheckState::Failed => "failure",
        CheckState::Skipped => "skipped",
        CheckState::Cancelled => "cancelled",
        CheckState::Pending | CheckState::Unknown => "pending",
    }
}

/// Paseo's `canAddPullRequestCheckLogsToChat`: a failed check.
#[must_use]
pub const fn can_add_check(run: &CheckRun) -> bool {
    matches!(run.state, CheckState::Failed)
}

/// A failed check, with what `pr.check` read of it when it could.
#[must_use]
pub fn check(about: &About<'_>, run: &CheckRun, report: Option<&CheckReport>) -> Attachment {
    let check_url = run.url.as_ref().map_or(about.url, |url| url.as_str());
    let mut lines = about.lines("GitHub pull request check");
    lines.push(format!("Check: {}", run.name.as_str()));
    lines.push(format!("Status: {}", check_status(run.state)));
    if let Some(report) = report.filter(|report| report.state != CheckState::Pending) {
        lines.push(format!("Conclusion: {}", check_status(report.state)));
    }
    lines.push(format!("Check URL: {check_url}"));
    let details = report.and_then(|report| report.details_url.as_ref().or(report.url.as_ref()));
    if let Some(details) = details {
        lines.push(format!("Details URL: {}", details.as_str()));
    }
    if let Some(report) = report {
        report_lines(&mut lines, report);
    }
    let id = run.run.map_or_else(
        || format!("{}:check:{}", about.number, run.name.as_str()),
        |id| format!("{}:check-run:{}", about.number, id.get()),
    );
    Attachment {
        kind: AttachKind::Check,
        id,
        title: run.name.as_str().to_owned(),
        text: lines.join("\n"),
        url: Some(details.map_or(check_url, |url| url.as_str()).to_owned()),
    }
}

/// The output, the annotations, the failed jobs and their logs.
fn report_lines(lines: &mut Vec<String>, report: &CheckReport) {
    if let Some(output) = &report.output {
        if let Some(title) = &output.title {
            lines.push(format!("Output title: {}", title.as_str()));
        }
        if let Some(summary) = &output.summary {
            lines.push(format!("Output summary: {}", summary.as_str()));
        }
        if let Some(text) = &output.text {
            lines.push("Output text:".to_owned());
            lines.push(text.as_str().to_owned());
        }
    }
    if !report.annotations.is_empty() {
        lines.push(String::new());
        lines.push("Annotations:".to_owned());
        for note in &report.annotations {
            let level = match note.level {
                AnnotationLevel::Notice => " notice",
                AnnotationLevel::Warning => " warning",
                AnnotationLevel::Failure => " failure",
                AnnotationLevel::Unknown => "",
            };
            let message = note.message.as_str();
            let message = if message.is_empty() {
                String::new()
            } else {
                format!(": {message}")
            };
            lines.push(format!(
                "- {}:{}-{}{level}{message}",
                note.path.as_str(),
                note.start_line,
                note.end_line
            ));
        }
    }
    if !report.failed_jobs.is_empty() {
        lines.push(String::new());
        lines.push("Failed jobs:".to_owned());
        for job in &report.failed_jobs {
            lines.push(format!(
                "- {}: {}",
                job.name.as_str(),
                check_status(job.state)
            ));
            if let Some(url) = &job.url {
                lines.push(format!("  {}", url.as_str()));
            }
            if let Some(log) = &job.log {
                lines.push("  ```".to_owned());
                lines.extend(log.text.as_str().split('\n').map(|line| format!("  {line}")));
                lines.push("  ```".to_owned());
                if log.cut {
                    lines.push("  Log tail truncated to the latest capped lines.".to_owned());
                }
            }
        }
    }
    if report.truncated {
        lines.push(String::new());
        lines.push("Note: Check details were truncated by GitHub/API or local caps.".to_owned());
    }
}

#[cfg(test)]
mod tests;
