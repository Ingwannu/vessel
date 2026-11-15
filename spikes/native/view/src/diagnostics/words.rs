//! Whole translated recovery sentences, independent of the English report schema.
use super::collection::{Problem, Progress};
use crate::localization::Translator;

/// A closed failure becomes one complete recovery sentence.
#[must_use]
pub fn problem(problem: Problem, words: &'static Translator) -> &'static str {
    let words = words.diagnostic_flow();
    match problem {
        Problem::Offline => words.offline(),
        Problem::Busy => words.busy(),
        Problem::Unavailable => words.unavailable(),
        Problem::Unsupported => words.unsupported(),
        Problem::Forbidden => words.forbidden(),
        Problem::TimedOut => words.timed_out(),
        Problem::InvalidReply => words.invalid_reply(),
        Problem::Failed => words.failed(),
        Problem::TooLarge => words.too_large(),
        Problem::Exhausted => words.exhausted(),
    }
}
/// A row status is translated without parsing or inspecting report text.
#[must_use]
pub fn progress(progress: &Progress, words: &'static Translator) -> &'static str {
    let labels = words.diagnostic_flow();
    match progress {
        Progress::Pending => labels.pending(),
        Progress::Running => labels.running(),
        Progress::Done(_) => labels.done(),
        Progress::Partial(_) => labels.collected_partial(),
        Progress::Failed(failure) => problem(*failure, words),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::localization::{ENGLISH, Locale};
    #[test]
    fn recovery_sentences_are_complete_in_both_languages() {
        let en = Translator::get(ENGLISH).unwrap();
        let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
        for (failure, english, korean) in [
            (
                Problem::Offline,
                "The host is not connected. Reconnect and refresh.",
                "머신이 연결되어 있지 않습니다. 다시 연결한 뒤 새로 고침하세요.",
            ),
            (
                Problem::Unavailable,
                "This diagnostic source is not available in this build.",
                "이 버전에서는 이 진단 정보를 제공하지 않습니다.",
            ),
            (
                Problem::TooLarge,
                "The combined report is too large. Run a report for one host.",
                "통합 보고서가 너무 큽니다. 머신 한 대의 보고서를 생성하세요.",
            ),
        ] {
            assert_eq!(problem(failure, en), english);
            assert_eq!(problem(failure, ko), korean);
        }
    }
}
