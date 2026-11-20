//! One check run read whole for the pane (ADR 0088, `pr.check`): Paseo's
//! `getCheckDetails` on GitHub. The check run, one page of its
//! annotations, its workflow run's jobs (one page), and the log tail of
//! each failed job (at most five).
//!
//! [Decision Log]
//! - Purpose: "Add logs to chat" on a failed check hands the agent what
//!   failed, where and the end of its log.
//! - Choice: REST, as Paseo's `gh api` calls, with its page sizes (20
//!   annotations, 100 jobs, 5 failed jobs). A job's log is read through
//!   `Http::log_tail` (the one token-free redirect, ADR 0088); a log that
//!   cannot be read leaves that job without one, it does not fail the
//!   check (Paseo still attaches what it has).

use chrono::{DateTime, Utc};
use reqwest::Method;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use vessel_proto::forge::{
    Annotation, AnnotationLevel, CheckName, CheckOutput, CheckReport, FailedJob, ForgeHost,
    ForgePath, ForgeUrl, LogTail, PullBody, PullTitle, RepoRef, RunId,
};

use super::{GitHub, decode, facts, no_access};
use crate::http::{Call, Resource};
use crate::provider::{ForgeError, Priority};

/// Annotations read (Paseo's `CHECK_ANNOTATION_PAGE_MAX`, one page).
const ANNOTATIONS: usize = CheckReport::MAX_ANNOTATIONS;
/// Jobs read of a workflow run (Paseo's `ACTIONS_JOB_PAGE_MAX`, one page).
const JOBS: u64 = 100;

#[derive(Debug, Deserialize)]
struct Run {
    name: String,
    status: String,
    #[serde(default)]
    conclusion: Option<String>,
    #[serde(default)]
    html_url: Option<String>,
    #[serde(default)]
    details_url: Option<String>,
    #[serde(default)]
    output: Option<Output>,
}

#[derive(Debug, Deserialize)]
struct Output {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Note {
    path: String,
    start_line: u32,
    end_line: u32,
    annotation_level: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Jobs {
    #[serde(default)]
    total_count: u64,
    #[serde(default)]
    jobs: Vec<Job>,
}

#[derive(Debug, Deserialize)]
struct Job {
    id: u64,
    name: String,
    status: String,
    #[serde(default)]
    conclusion: Option<String>,
    #[serde(default)]
    html_url: Option<String>,
    #[serde(default)]
    completed_at: Option<DateTime<Utc>>,
}

impl GitHub {
    /// The check run `run` of `repo`, read whole.
    pub(super) async fn check_report(
        &self,
        repo: &RepoRef,
        run: RunId,
        workflow_run: Option<RunId>,
    ) -> Result<CheckReport, ForgeError> {
        let base = format!("repos/{}", repo.path);
        let found: Run = self
            .get_json(
                &repo.host,
                &format!("{base}/check-runs/{}", run.get()),
                "a check run",
            )
            .await?;
        let notes: Vec<Note> = self
            .get_json(
                &repo.host,
                &format!(
                    "{base}/check-runs/{}/annotations?per_page={ANNOTATIONS}",
                    run.get()
                ),
                "a check run's annotations",
            )
            .await?;
        let mut truncated = notes.len() >= ANNOTATIONS;
        let annotations = notes
            .iter()
            .take(ANNOTATIONS)
            .filter_map(annotation)
            .collect();
        let mut failed_jobs = Vec::new();
        if let Some(workflow_run) = workflow_run {
            let jobs: Jobs = self
                .get_json(
                    &repo.host,
                    &format!(
                        "{base}/actions/runs/{}/jobs?per_page={JOBS}",
                        workflow_run.get()
                    ),
                    "a workflow run's jobs",
                )
                .await?;
            let failed: Vec<&Job> = jobs.jobs.iter().filter(|job| failed_job(job)).collect();
            truncated |= jobs.total_count > JOBS || failed.len() > CheckReport::MAX_FAILED_JOBS;
            for job in failed.into_iter().take(CheckReport::MAX_FAILED_JOBS) {
                let Ok(id) = RunId::new(job.id) else {
                    continue;
                };
                let log = self.job_log(repo, &base, id).await;
                failed_jobs.push(FailedJob {
                    job: id,
                    name: CheckName::lossy(&job.name),
                    state: facts::check_run(
                        &job.status.to_ascii_uppercase(),
                        job.conclusion
                            .as_deref()
                            .map(str::to_ascii_uppercase)
                            .as_deref(),
                    ),
                    url: job.html_url.as_deref().and_then(ForgeUrl::lossy),
                    completed_at: job.completed_at,
                    log,
                });
            }
        }
        Ok(CheckReport {
            run,
            name: CheckName::lossy(&found.name),
            state: facts::check_run(
                &found.status.to_ascii_uppercase(),
                found
                    .conclusion
                    .as_deref()
                    .map(str::to_ascii_uppercase)
                    .as_deref(),
            ),
            url: found.html_url.as_deref().and_then(ForgeUrl::lossy),
            details_url: found.details_url.as_deref().and_then(ForgeUrl::lossy),
            output: found.output.map(|output| CheckOutput {
                title: output.title.as_deref().map(PullTitle::lossy),
                summary: output.summary.as_deref().map(PullBody::lossy),
                text: output.text.as_deref().map(PullBody::lossy),
            }),
            annotations,
            failed_jobs,
            truncated,
        })
    }

    /// A REST read of `path` on `host`'s API, as a person's.
    async fn get_json<T: DeserializeOwned>(
        &self,
        host: &ForgeHost,
        path: &str,
        what: &'static str,
    ) -> Result<T, ForgeError> {
        let url = self.rest(host, path)?;
        match self
            .send(host, super::Ask::get(url, Priority::Person))
            .await?
        {
            crate::http::Reply::Body { status, body } if status.is_success() => decode(&body, what),
            crate::http::Reply::Body { status, .. } => {
                Err(no_access(status).unwrap_or(ForgeError::Unexpected { what }))
            }
            crate::http::Reply::NotModified => Err(ForgeError::Unexpected { what }),
        }
    }

    /// A failed job's log tail; `None` when it cannot be read (expired,
    /// refused, or not one https address).
    async fn job_log(&self, repo: &RepoRef, base: &str, job: RunId) -> Option<LogTail> {
        let url = self
            .rest(
                &repo.host,
                &format!("{base}/actions/jobs/{}/logs", job.get()),
            )
            .ok()?;
        let token = self.token(&repo.host).await.ok()?;
        let tail = self
            .http
            .log_tail(Call {
                method: Method::GET,
                url,
                host: &repo.host,
                token: &token,
                resource: Resource::Core,
                priority: Priority::Person,
                conditional: false,
                json: None,
            })
            .await
            .inspect_err(|error| tracing::debug!(%error, "a job's log could not be read"))
            .ok()?;
        Some(LogTail::of(&tail.bytes, tail.cut))
    }
}

/// Paseo's `isFailedActionsJob`.
fn failed_job(job: &Job) -> bool {
    matches!(
        job.conclusion.as_deref(),
        Some("failure" | "cancelled" | "timed_out" | "action_required")
    )
}

/// An annotation, `None` when its path cannot be a wire value.
fn annotation(note: &Note) -> Option<Annotation> {
    Some(Annotation {
        path: ForgePath::lossy(&note.path)?,
        start_line: note.start_line,
        end_line: note.end_line,
        level: match note.annotation_level.as_str() {
            "notice" => AnnotationLevel::Notice,
            "warning" => AnnotationLevel::Warning,
            "failure" => AnnotationLevel::Failure,
            _ => AnnotationLevel::Unknown,
        },
        message: PullBody::lossy(&note.message),
        title: note
            .title
            .as_deref()
            .filter(|title| !title.trim().is_empty())
            .map(PullTitle::lossy),
    })
}
