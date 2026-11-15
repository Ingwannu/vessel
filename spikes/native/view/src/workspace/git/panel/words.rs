//! Prepared Fluent words for comparison surfaces. Typed failures select a
//! recovery; git's diagnostic never selects or replaces the app's sentence.

use crate::localization::{GitPanel, Translator};
use crate::workspace::problem::Problem;
use vessel_proto::ErrorCode;

use super::{ComparisonMode, EmptyAction};

/// Honest recovery after a failed read, independent of diagnostic language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadIssue {
    /// No command reached the node, so retry cannot duplicate a write.
    Unsent,
    /// Reconnect before asking again.
    Connection,
    /// The device's read grant needs attention.
    Access,
    /// An unassigned/newer call cannot be approximated by a different scope.
    Version,
    /// An unsupported call is not proof that the folder is not a repository.
    Unsupported,
    /// The registered folder is no longer available.
    Missing,
    /// A transient refusal allows a person to retry.
    Unavailable,
    /// Unclassified read failure retains its diagnostic separately.
    Failed,
}

impl ReadIssue {
    /// Map only machine-readable outcomes, exhaustively.
    #[must_use]
    pub fn of(problem: &Problem) -> Self {
        match problem {
            Problem::Unsent => Self::Unsent,
            Problem::Older => Self::Version,
            Problem::Gone(_) => Self::Connection,
            Problem::Node { code, .. } => match code {
                ErrorCode::Unauthorized | ErrorCode::Forbidden => Self::Access,
                ErrorCode::ProtocolMismatch => Self::Version,
                ErrorCode::Unsupported => Self::Unsupported,
                ErrorCode::NotFound => Self::Missing,
                ErrorCode::Unavailable => Self::Unavailable,
                ErrorCode::InvalidRequest
                | ErrorCode::Conflict
                | ErrorCode::HarnessNotInstalled
                | ErrorCode::SpawnFailed
                | ErrorCode::Internal => Self::Failed,
            },
        }
    }

    /// Borrow a whole recovery sentence from the prepared language catalog.
    #[must_use]
    pub fn words(self, catalog: &Translator) -> &str {
        let words = catalog.git_panel();
        match self {
            Self::Unsent => words.problem_unsent(),
            Self::Connection => words.problem_connection(),
            Self::Access => words.problem_access(),
            Self::Version => words.problem_version(),
            Self::Unsupported => words.problem_unsupported(),
            Self::Missing => words.problem_missing(),
            Self::Unavailable => words.problem_unavailable(),
            Self::Failed => words.problem_failed(),
        }
    }
}

impl<'a> GitPanel<'a> {
    /// One label per place; no conditionals or strings in drawing code.
    #[must_use]
    pub fn comparison(self, mode: ComparisonMode) -> &'a str {
        match mode {
            ComparisonMode::All => self.mode_all(),
            ComparisonMode::Uncommitted => self.mode_uncommitted(),
            ComparisonMode::Committed => self.mode_committed(),
        }
    }

    /// The empty-state action comes from the pure body projection.
    #[must_use]
    pub fn empty_action(self, action: EmptyAction) -> &'a str {
        match action {
            EmptyAction::Uncommitted => self.see_uncommitted(),
            EmptyAction::Committed => self.see_committed(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::localization::{ENGLISH, LanguagePreference, resolve_language};

    #[test]
    fn every_recovery_is_a_complete_sentence_in_english_and_korean() {
        let en = Translator::get(ENGLISH).unwrap();
        let ko = Translator::get(resolve_language(
            LanguagePreference::System,
            Some("ko"),
            None,
        ))
        .unwrap();
        for (issue, english, korean) in [
            (
                ReadIssue::Unsent,
                "Could not send. Try again.",
                "보내지 못했습니다. 다시 시도하세요.",
            ),
            (
                ReadIssue::Connection,
                "The machine is disconnected. Reconnect and refresh the changes.",
                "머신의 연결이 끊겼습니다. 다시 연결하고 변경 사항을 새로고침하세요.",
            ),
            (
                ReadIssue::Access,
                "This device cannot read changes on this machine. Check its access.",
                "이 기기에서 머신의 변경 사항을 읽을 수 없습니다. 접근 권한을 확인하세요.",
            ),
            (
                ReadIssue::Version,
                "Update Vessel on this device and the machine to view these changes.",
                "변경 사항을 보려면 이 기기와 머신의 Vessel을 업데이트하세요.",
            ),
            (
                ReadIssue::Unsupported,
                "This machine cannot provide these changes. Check Git and Vessel on the machine, then refresh.",
                "이 머신에서 변경 사항을 제공할 수 없습니다. 머신의 Git과 Vessel을 확인하고 새로고침하세요.",
            ),
            (
                ReadIssue::Missing,
                "This folder is no longer available. Open another workspace.",
                "이 폴더를 더 이상 사용할 수 없습니다. 다른 워크스페이스를 여세요.",
            ),
            (
                ReadIssue::Unavailable,
                "The machine cannot read these changes right now. Refresh and try again.",
                "지금은 머신에서 변경 사항을 읽을 수 없습니다. 새로고침하고 다시 시도하세요.",
            ),
            (
                ReadIssue::Failed,
                "The machine could not read these changes. Refresh and try again.",
                "머신에서 변경 사항을 읽지 못했습니다. 새로고침하고 다시 시도하세요.",
            ),
        ] {
            assert_eq!(issue.words(en), english);
            assert_eq!(issue.words(ko), korean);
        }
    }

    #[test]
    fn errors_never_use_english_git_output_to_decide_the_recovery() {
        for (code, expected) in [
            (ErrorCode::Unauthorized, ReadIssue::Access),
            (ErrorCode::Forbidden, ReadIssue::Access),
            (ErrorCode::ProtocolMismatch, ReadIssue::Version),
            (ErrorCode::Unsupported, ReadIssue::Unsupported),
            (ErrorCode::NotFound, ReadIssue::Missing),
            (ErrorCode::Unavailable, ReadIssue::Unavailable),
            (ErrorCode::InvalidRequest, ReadIssue::Failed),
            (ErrorCode::Conflict, ReadIssue::Failed),
            (ErrorCode::HarnessNotInstalled, ReadIssue::Failed),
            (ErrorCode::SpawnFailed, ReadIssue::Failed),
            (ErrorCode::Internal, ReadIssue::Failed),
        ] {
            assert_eq!(
                ReadIssue::of(&Problem::Node {
                    code,
                    message: "unrelated diagnostic".to_owned()
                }),
                expected
            );
        }
        assert_eq!(ReadIssue::of(&Problem::Older), ReadIssue::Version);
        assert_eq!(
            ReadIssue::of(&Problem::Gone("transport diagnostic".to_owned())),
            ReadIssue::Connection
        );
        assert_eq!(ReadIssue::of(&Problem::Unsent), ReadIssue::Unsent);
    }
}
