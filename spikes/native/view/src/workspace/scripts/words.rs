//! Typed failures resolve to complete translated notices; raw node or file
//! errors may contain command text and never become a browser destination.
use super::{Issue, PreviewProblem, RunState};
use crate::localization::Translator;
use vessel_proto::scripts::ScriptRefusal;

/// A retained state's short label; numeric exit codes are a separate value.
#[must_use]
pub fn status(words: &Translator, state: RunState) -> &str {
    let words = words.scripts();
    match state {
        RunState::Idle => words.idle(),
        RunState::Starting => words.starting(),
        RunState::Running => words.running(),
        RunState::Backoff => words.backoff(),
        RunState::Exited(_) => words.exited(),
        RunState::Failed => words.failed(),
        RunState::Stopped => words.stopped(),
        RunState::Unknown => words.unknown(),
    }
}
/// Complete failure text, independent from platform diagnostics and secrets.
#[must_use]
pub fn issue<'a>(words: &'a Translator, issue: &Issue) -> &'a str {
    let words = words.scripts();
    match issue {
        Issue::Call(_) => words.call_failed(),
        Issue::Approval => words.approval(),
        Issue::Exhausted => words.exhausted(),
        Issue::Retention => words.retained_limit(),
        Issue::Preview(problem) => match problem {
            PreviewProblem::MissingHost => words.missing_host(),
            PreviewProblem::InvalidHost => words.invalid_host(),
            PreviewProblem::ZeroPort => words.invalid_port(),
            PreviewProblem::RemoteLoopback => words.remote_loopback(),
            PreviewProblem::Stale => words.stale(),
        },
        Issue::Refused(reason) => match reason {
            ScriptRefusal::NoProject => words.no_project(),
            ScriptRefusal::NoSuchScript => words.missing_script(),
            ScriptRefusal::NoSuchDir => words.missing_dir(),
            ScriptRefusal::InvalidFile { .. } => words.invalid_file(),
            ScriptRefusal::AlreadyRunning { .. } => words.already_running(),
            ScriptRefusal::Unknown => words.refused(),
        },
    }
}
