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
    let labels = words.scripts();
    match issue {
        Issue::Call(_) => labels.call_failed(),
        Issue::Approval => labels.approval(),
        Issue::Exhausted => labels.exhausted(),
        Issue::Retention => labels.retained_limit(),
        Issue::Preview(problem) => match problem {
            PreviewProblem::MissingHost => labels.missing_host(),
            PreviewProblem::InvalidHost => labels.invalid_host(),
            PreviewProblem::ZeroPort => labels.invalid_port(),
            PreviewProblem::RemoteLoopback => labels.remote_loopback(),
            PreviewProblem::Stale => labels.stale(),
        },
        Issue::Refused(reason) => refusal(words, reason),
    }
}

fn refusal<'a>(words: &'a Translator, reason: &ScriptRefusal) -> &'a str {
    let labels = words.scripts();
    match reason {
        ScriptRefusal::NoProject => labels.no_project(),
        ScriptRefusal::NoSuchScript => labels.missing_script(),
        ScriptRefusal::NoSuchDir => labels.missing_dir(),
        ScriptRefusal::InvalidFile { .. } => labels.invalid_file(),
        ScriptRefusal::AlreadyRunning { .. } => labels.already_running(),
        ScriptRefusal::Unknown => labels.refused(),
    }
}

/// A retained receipt reports only that operation's answer, never current health.
#[must_use]
pub fn receipt<'a>(words: &'a Translator, write: &super::RetiredWrite) -> &'a str {
    use vessel_proto::scripts::ScriptRunOutcome;
    let labels = words.scripts();
    match write.receipt() {
        None => labels.receipt_pending(),
        Some(Err(_problem)) => labels.receipt_uncertain(),
        Some(Ok(None)) => labels.receipt_acknowledged(),
        Some(Ok(Some(ScriptRunOutcome::Started { .. }))) => labels.receipt_started(),
        Some(Ok(Some(ScriptRunOutcome::NeedsApproval { .. }))) => labels.approval(),
        Some(Ok(Some(ScriptRunOutcome::Refused { reason }))) => refusal(words, reason),
    }
}
