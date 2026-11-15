//! Whole translated recovery sentences, independent of the English report schema.
use super::collection::{Problem, Progress};
use crate::localization::Translator;

/// A closed failure becomes one complete recovery sentence.
#[must_use]
pub fn problem(problem: Problem, words: &'static Translator) -> &'static str {
    let words = words.diagnostic_flow();
    match problem {
        Problem::Offline => words.offline(),
        Problem::ConnectionChanged => words.connection_changed(),
        Problem::Busy => words.busy(),
        Problem::Unavailable => words.unavailable(),
        Problem::Stopping => words.stopping(),
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
    use crate::diagnostics::{
        DaemonSnapshot,
        collection::{HostResult, NodeFacts},
    };
    use crate::localization::{ENGLISH, Locale};
    use vessel_proto::{NodeId, NodeState, ProtocolVersion};
    #[test]
    fn every_recovery_sentence_is_complete_in_both_languages() {
        let en = Translator::get(ENGLISH).unwrap();
        let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
        for (failure, english, korean) in [
            (
                Problem::Offline,
                "The host is not connected. Reconnect and refresh.",
                "머신이 연결되어 있지 않습니다. 다시 연결한 뒤 새로 고침하세요.",
            ),
            (
                Problem::ConnectionChanged,
                "The host connection changed. Run the report again.",
                "머신 연결이 변경되었습니다. 보고서를 다시 생성하세요.",
            ),
            (
                Problem::Busy,
                "Diagnostics is busy. Try again when the current read finishes.",
                "진단이 진행 중입니다. 현재 조회가 끝나면 다시 시도하세요.",
            ),
            (
                Problem::Stopping,
                "The host is stopping. Reconnect before collecting diagnostics again.",
                "머신이 종료 중입니다. 다시 연결한 뒤 진단 정보를 수집하세요.",
            ),
            (
                Problem::Unavailable,
                "This diagnostic source is not available in this build.",
                "이 버전에서는 이 진단 정보를 제공하지 않습니다.",
            ),
            (
                Problem::Unsupported,
                "This host does not support this diagnostic.",
                "이 머신은 이 진단을 지원하지 않습니다.",
            ),
            (
                Problem::Forbidden,
                "This device cannot read this diagnostic. Check its host access.",
                "이 기기에서는 이 진단을 조회할 수 없습니다. 머신 접근 권한을 확인하세요.",
            ),
            (
                Problem::TimedOut,
                "The diagnostic timed out. Refresh to try again.",
                "진단 시간이 초과되었습니다. 새로 고침하여 다시 시도하세요.",
            ),
            (
                Problem::InvalidReply,
                "The host returned an invalid diagnostic. Refresh after reconnecting.",
                "머신에서 올바르지 않은 진단 정보를 받았습니다. 다시 연결한 뒤 새로 고침을 누르세요.",
            ),
            (
                Problem::Failed,
                "The diagnostic could not be collected. Refresh to try again.",
                "진단 정보를 수집하지 못했습니다. 새로 고침을 눌러 다시 시도하세요.",
            ),
            (
                Problem::TooLarge,
                "The combined report is too large. Run a report for one host.",
                "통합 보고서가 너무 큽니다. 머신 한 대의 보고서를 생성하세요.",
            ),
            (
                Problem::Exhausted,
                "No more diagnostic requests can be issued. Reopen the app.",
                "진단 요청을 더 이상 보낼 수 없습니다. 앱을 종료한 뒤 다시 실행하세요.",
            ),
        ] {
            assert_eq!(problem(failure, en), english);
            assert_eq!(problem(failure, ko), korean);
            assert_eq!(progress(&Progress::Failed(failure), en), english);
            assert_eq!(progress(&Progress::Failed(failure), ko), korean);
        }
    }
    #[test]
    fn every_progress_status_has_exact_complete_localized_words() {
        let en = Translator::get(ENGLISH).unwrap();
        let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
        let id = NodeId::random();
        let partial = HostResult {
            admitted: id,
            facts: Ok(NodeFacts {
                node: id,
                protocol: ProtocolVersion::new(13),
                state: NodeState::Serving,
            }),
            daemon: DaemonSnapshot::Unavailable(Problem::Unavailable),
        };
        let full = HostResult {
            daemon: crate::diagnostics::fixture::complete(id),
            ..partial.clone()
        };
        assert!(full.daemon.complete());
        for (state, english, korean) in [
            (Progress::Pending, "Waiting", "대기 중"),
            (Progress::Running, "Collecting", "수집 중"),
            (Progress::Done(full), "Collected", "수집 완료"),
            (
                Progress::Partial(partial),
                "Collected with unavailable sections",
                "일부 항목 수집 완료",
            ),
        ] {
            assert_eq!(progress(&state, en), english);
            assert_eq!(progress(&state, ko), korean);
        }
    }
}
