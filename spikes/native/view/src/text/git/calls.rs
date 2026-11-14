//! Call failures use typed codes for guidance. The node's diagnostic is
//! retained beneath it, so changing Git's wording cannot change meaning.

use vessel_proto::ErrorCode;

use crate::workspace::problem::Problem;
use crate::{GitFailureText, Lang};

impl Lang {
    /// A git read/write call that did not produce its typed outcome.
    #[must_use]
    pub fn git_call_failure(self, problem: &Problem) -> GitFailureText {
        let (code, detail) = match problem {
            Problem::Node { code, message } => (Some(*code), Some(message.clone())),
            Problem::Older => (Some(ErrorCode::ProtocolMismatch), None),
            Problem::Gone(detail) => (None, Some(detail.clone())),
            // Nothing reached the machine, so there is no node detail and
            // no reconnect advice: the lead's words for an act the app's
            // own queue gave back (`Problem::Unsent`), split at the sentence.
            Problem::Unsent => return self.git_unsent(),
        };
        let (title, next) = self.git_call_words(code);
        GitFailureText {
            title: title.to_owned(),
            next: next.to_owned(),
            detail,
        }
    }

    /// The two sentences for a call's code (none: the machine is gone).
    const fn git_call_words(self, code: Option<ErrorCode>) -> (&'static str, &'static str) {
        match (self, code) {
            (Self::En, None) => (
                "The app could not complete this request to the machine.",
                "Reconnect the machine, then try again.",
            ),
            (Self::Ko, None) => (
                "이 머신에 대한 요청을 완료하지 못했습니다.",
                "머신을 다시 연결한 뒤 시도하세요.",
            ),
            (Self::En, Some(ErrorCode::ProtocolMismatch)) => (
                "This version of the machine cannot handle this Git action.",
                "Update Vessel on this machine and in the app, then reconnect.",
            ),
            (Self::Ko, Some(ErrorCode::ProtocolMismatch)) => (
                "이 머신의 버전은 이 Git 동작을 처리하지 못합니다.",
                "머신과 앱의 Vessel을 업데이트한 뒤 다시 연결하세요.",
            ),
            (Self::En, Some(ErrorCode::Unauthorized)) => (
                "The machine did not accept this app's sign-in.",
                "Pair this app with the machine again, then try again.",
            ),
            (Self::Ko, Some(ErrorCode::Unauthorized)) => (
                "머신이 이 앱의 로그인을 받아들이지 않았습니다.",
                "이 앱과 머신을 다시 페어링한 뒤 시도하세요.",
            ),
            (Self::En, Some(ErrorCode::Forbidden)) => (
                "This app does not have permission for this Git action.",
                "Pair again with permission to change the repository.",
            ),
            (Self::Ko, Some(ErrorCode::Forbidden)) => (
                "이 앱에 이 Git 동작의 권한이 없습니다.",
                "저장소를 변경할 권한으로 다시 페어링하세요.",
            ),
            (Self::En, Some(ErrorCode::NotFound)) => (
                "The repository could not be found on this machine.",
                "Check the project's folder in Settings, then refresh this view.",
            ),
            (Self::Ko, Some(ErrorCode::NotFound)) => (
                "이 머신에서 저장소를 찾지 못했습니다.",
                "설정에서 프로젝트 폴더를 확인한 뒤 이 화면을 새로 고치세요.",
            ),
            (Self::En, Some(ErrorCode::Conflict)) => (
                "The repository changed before this action could finish.",
                "Refresh its status, check your changes, and try again.",
            ),
            (Self::Ko, Some(ErrorCode::Conflict)) => (
                "동작이 끝나기 전에 저장소가 바뀌었습니다.",
                "상태를 새로 읽고 변경을 확인한 뒤 다시 시도하세요.",
            ),
            (Self::En, Some(ErrorCode::Unsupported)) => (
                "Git cannot perform this action in this folder.",
                "Check that Git is installed and the folder is a Git repository.",
            ),
            (Self::Ko, Some(ErrorCode::Unsupported)) => (
                "Git이 이 폴더에서 이 동작을 수행할 수 없습니다.",
                "Git이 설치되어 있고 이 폴더가 Git 저장소인지 확인하세요.",
            ),
            (Self::En, Some(ErrorCode::Unavailable)) => (
                "The repository or machine is busy.",
                "Wait for the other operation to finish, then try again.",
            ),
            (Self::Ko, Some(ErrorCode::Unavailable)) => (
                "저장소나 머신이 다른 작업 중입니다.",
                "다른 동작이 끝날 때까지 기다린 뒤 다시 시도하세요.",
            ),
            (
                Self::En,
                Some(
                    ErrorCode::InvalidRequest
                    | ErrorCode::Internal
                    | ErrorCode::HarnessNotInstalled
                    | ErrorCode::SpawnFailed,
                ),
            ) => (
                "The machine could not complete this Git action.",
                "Read the details below, correct the problem, and try again.",
            ),
            (
                Self::Ko,
                Some(
                    ErrorCode::InvalidRequest
                    | ErrorCode::Internal
                    | ErrorCode::HarnessNotInstalled
                    | ErrorCode::SpawnFailed,
                ),
            ) => (
                "머신이 이 Git 동작을 완료하지 못했습니다.",
                "아래 상세 출력을 읽고 문제를 고친 뒤 다시 시도하세요.",
            ),
        }
    }

    /// The app's link did not take the call: the workspace's own unsent
    /// sentence, split into its two sentences.
    fn git_unsent(self) -> GitFailureText {
        let (title, next) = match self {
            Self::En => ("Could not send.", "Try again."),
            Self::Ko => ("보내지 못했습니다.", "다시 시도하세요."),
        };
        GitFailureText {
            title: title.to_owned(),
            next: next.to_owned(),
            detail: None,
        }
    }
}
