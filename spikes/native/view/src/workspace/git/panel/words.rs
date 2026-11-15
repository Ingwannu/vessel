//! Prepared Fluent words for comparison surfaces. Typed failures select a
//! recovery; git's diagnostic never selects or replaces the app's sentence.

use crate::localization::{GitPanel, Translator};
use crate::workspace::problem::Problem;
use vessel_proto::ErrorCode;

use super::{ComparisonMode, EmptyAction, ReadFailure};
use crate::workspace::git_quick::GitAction;

/// Honest recovery after a failed read, independent of diagnostic language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadIssue {
    /// No comparison base exists; never present it as an empty successful diff.
    NoBase,
    /// The complete bounded comparison cannot fit the preview.
    TooLarge,
    /// Root admission must come from the canonical workspace registry.
    Workspace,
    /// Restart before the lifetime request issuer can safely allocate again.
    Keys,
    /// No command reached the node, so retry cannot duplicate a write.
    Unsent,
    /// Reconnect before asking again.
    Connection,
    /// The app's read admission is full, not a node refusal.
    Capacity,
    /// A deadline does not prove the physical connection closed.
    TimedOut,
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
    /// The selected connection changed; never silently choose another route.
    DestinationChanged,
    /// The returned body belongs to another path and cannot be shown here.
    UnexpectedFile,
    /// Stored and requested refs differ; both facts remain on the failure.
    BaseMismatch,
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

    /// Include local validation outcomes without pretending they came from git.
    #[must_use]
    pub fn for_failure(failure: &ReadFailure) -> Self {
        match failure {
            ReadFailure::NoBase => Self::NoBase,
            ReadFailure::TooLarge => Self::TooLarge,
            ReadFailure::WorkspaceUnavailable => Self::Workspace,
            ReadFailure::KeysExhausted => Self::Keys,
            ReadFailure::Call(problem) => Self::of(problem),
            ReadFailure::ComparisonUnavailable => Self::Version,
            ReadFailure::DestinationChanged | ReadFailure::UnexpectedNode { .. } => {
                Self::DestinationChanged
            }
            ReadFailure::UnexpectedFile { .. } => Self::UnexpectedFile,
            ReadFailure::UnexpectedSession { .. } => Self::Failed,
            ReadFailure::DestinationClosed => Self::Connection,
            ReadFailure::Capacity => Self::Capacity,
            ReadFailure::TimedOut => Self::TimedOut,
            ReadFailure::BaseMismatch { .. } => Self::BaseMismatch,
        }
    }

    /// Pending capabilities are information; failed reads retain an error signal.
    #[must_use]
    pub const fn variant(self) -> crate::alert::Variant {
        match self {
            Self::NoBase | Self::TooLarge | Self::Workspace | Self::Version | Self::Unsupported => {
                crate::alert::Variant::Info
            }
            Self::Keys
            | Self::Unsent
            | Self::Connection
            | Self::Capacity
            | Self::TimedOut
            | Self::Access
            | Self::Missing
            | Self::Unavailable
            | Self::DestinationChanged
            | Self::UnexpectedFile
            | Self::BaseMismatch
            | Self::Failed => crate::alert::Variant::Error,
        }
    }

    /// Borrow a whole recovery sentence from the prepared language catalog.
    #[must_use]
    pub fn words(self, catalog: &Translator) -> &str {
        let words = catalog.git_panel();
        match self {
            Self::NoBase => words.problem_no_base(),
            Self::TooLarge => words.problem_too_large(),
            Self::Workspace => words.problem_workspace(),
            Self::Keys => words.problem_keys(),
            Self::Unsent => words.problem_unsent(),
            Self::Connection => words.problem_connection(),
            Self::Capacity => words.problem_capacity(),
            Self::TimedOut => words.problem_timeout(),
            Self::Access => words.problem_access(),
            Self::Version => words.problem_version(),
            Self::Unsupported => words.problem_unsupported(),
            Self::Missing => words.problem_missing(),
            Self::Unavailable => words.problem_unavailable(),
            Self::DestinationChanged => words.problem_destination(),
            Self::UnexpectedFile => words.problem_file(),
            Self::BaseMismatch => words.problem_base(),
            Self::Failed => words.problem_failed(),
        }
    }
}

impl ReadFailure {
    /// Retain every peer identity and diagnostic beneath the recovery sentence.
    /// Labels come from the catalog; raw values never choose recovery policy.
    #[must_use]
    pub fn detail(&self, catalog: &Translator) -> Option<String> {
        let words = catalog.git_panel();
        let pair = |expected: String, received: String| {
            format!(
                "{}\n{expected}\n{}\n{received}",
                words.detail_expected(),
                words.detail_received()
            )
        };
        match self {
            Self::Call(Problem::Node { message, .. } | Problem::Gone(message)) => {
                Some(message.clone())
            }
            Self::UnexpectedNode { expected, received } => {
                Some(pair(expected.to_string(), received.to_string()))
            }
            Self::UnexpectedSession {
                requested,
                received,
            } => Some(pair(requested.to_string(), received.to_string())),
            Self::UnexpectedFile {
                requested,
                received,
            } => Some(pair(requested.to_string(), received.to_string())),
            Self::BaseMismatch { stored, requested } => {
                let requested = match requested {
                    vessel_proto::workspace::WorktreeBase::Ref { name } => name.to_string(),
                    vessel_proto::workspace::WorktreeBase::Commit { id } => id.to_string(),
                };
                Some(format!(
                    "{}\n{stored}\n{}\n{requested}",
                    words.detail_stored_base(),
                    words.detail_requested_base()
                ))
            }
            Self::NoBase
            | Self::TooLarge
            | Self::Call(Problem::Unsent | Problem::Older)
            | Self::WorkspaceUnavailable
            | Self::KeysExhausted
            | Self::DestinationClosed
            | Self::Capacity
            | Self::TimedOut
            | Self::DestinationChanged
            | Self::ComparisonUnavailable => None,
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

    /// Paseo's brief action acknowledgement, separate from the full notice.
    #[must_use]
    pub fn quick_success(self, action: GitAction) -> &'a str {
        match action {
            GitAction::Commit => self.commit_success(),
            GitAction::Push => self.push_success(),
            GitAction::Pull => self.pull_success(),
            GitAction::Fetch => self.fetch_success(),
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
                ReadIssue::TooLarge,
                "This comparison is too large to preview. Narrow the comparison and try again.",
                "비교 결과가 너무 커서 미리 볼 수 없습니다. 비교 범위를 줄인 뒤 다시 시도하세요.",
            ),
            (
                ReadIssue::NoBase,
                "No base branch is available. Choose a base branch before comparing.",
                "비교할 기준 브랜치가 없습니다. 기준 브랜치를 선택한 뒤 다시 비교하세요.",
            ),
            (
                ReadIssue::Workspace,
                "Workspace repository information is unavailable. Open a registered workspace and try again.",
                "워크스페이스의 저장소 정보를 사용할 수 없습니다. 등록된 워크스페이스를 열고 다시 시도하세요.",
            ),
            (
                ReadIssue::Keys,
                "No more requests can be identified safely. Restart Vessel and try again.",
                "요청을 안전하게 구분할 수 없습니다. Vessel을 다시 시작하고 시도하세요.",
            ),
            (
                ReadIssue::Capacity,
                "Too many changes are being read at once. Try again.",
                "한 번에 너무 많은 변경을 읽고 있습니다. 다시 시도하세요.",
            ),
            (
                ReadIssue::TimedOut,
                "This read did not finish in time. Try again.",
                "읽기를 제시간에 마치지 못했습니다. 다시 시도하세요.",
            ),
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
                ReadIssue::DestinationChanged,
                "The connection changed. Reopen this workspace and refresh the changes.",
                "연결이 변경되었습니다. 이 워크스페이스를 다시 열고 변경 사항을 새로고침하세요.",
            ),
            (
                ReadIssue::UnexpectedFile,
                "The machine returned another file’s changes. Refresh and try again.",
                "머신에서 다른 파일의 변경 사항을 보냈습니다. 새로고침하고 다시 시도하세요.",
            ),
            (
                ReadIssue::BaseMismatch,
                "The stored and requested base branches differ. Check both branches and choose the comparison again.",
                "저장된 기준 브랜치와 요청한 기준 브랜치가 다릅니다. 두 브랜치를 확인하고 비교 기준을 다시 선택하세요.",
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

    #[test]
    fn ref_and_commit_base_refusals_select_the_same_neutral_recovery() {
        use vessel_proto::workspace::WorktreeBase;
        for requested in [
            WorktreeBase::Ref {
                name: "refs/heads/main".parse().unwrap(),
            },
            WorktreeBase::Commit {
                id: "a".repeat(40).parse().unwrap(),
            },
        ] {
            let failure = ReadFailure::BaseMismatch {
                stored: "refs/remotes/upstream/main".parse().unwrap(),
                requested,
            };
            assert_eq!(ReadIssue::for_failure(&failure), ReadIssue::BaseMismatch);
        }
    }

    #[test]
    fn quick_acknowledgements_match_the_reference_in_both_languages() {
        let en = Translator::get(ENGLISH).unwrap().git_panel();
        let ko = Translator::get(resolve_language(
            LanguagePreference::System,
            Some("ko"),
            None,
        ))
        .unwrap()
        .git_panel();
        for (action, english, korean) in [
            (GitAction::Commit, "Committed", "커밋됨"),
            (GitAction::Push, "Pushed", "푸시됨"),
            (GitAction::Pull, "Pulled", "가져옴"),
            (GitAction::Fetch, "Fetched", "원격 변경 가져옴"),
        ] {
            assert_eq!(en.quick_success(action), english);
            assert_eq!(ko.quick_success(action), korean);
        }
    }
    #[test]
    fn mismatch_details_keep_both_facts_with_whole_catalog_labels() {
        let en = Translator::get(ENGLISH).unwrap();
        let ko = Translator::get(resolve_language(
            LanguagePreference::System,
            Some("ko"),
            None,
        ))
        .unwrap();
        let mismatch = ReadFailure::UnexpectedFile {
            requested: "long/원래 파일.rs".parse().unwrap(),
            received: "other/actual.rs".parse().unwrap(),
        };
        assert_eq!(
            mismatch.detail(en).as_deref(),
            Some("Expected\nlong/원래 파일.rs\nReceived\nother/actual.rs")
        );
        assert_eq!(
            mismatch.detail(ko).as_deref(),
            Some("요청한 값\nlong/원래 파일.rs\n받은 값\nother/actual.rs")
        );
        let base = ReadFailure::BaseMismatch {
            stored: "refs/remotes/origin/main".parse().unwrap(),
            requested: vessel_proto::workspace::WorktreeBase::Commit {
                id: "a".repeat(40).parse().unwrap(),
            },
        };
        assert_eq!(
            base.detail(en).unwrap(),
            format!(
                "Stored base branch\nrefs/remotes/origin/main\nRequested comparison base\n{}",
                "a".repeat(40)
            )
        );
        assert_eq!(
            base.detail(ko).unwrap(),
            format!(
                "저장된 기준 브랜치\nrefs/remotes/origin/main\n요청한 비교 기준\n{}",
                "a".repeat(40)
            )
        );
        assert_eq!(ReadFailure::Capacity.detail(en), None);
        assert_eq!(ReadFailure::Call(Problem::Unsent).detail(ko), None);
    }
}
