//! Safe side-panel error text. A peer's arbitrary error message never
//! becomes a label, tooltip or log; the person's next step stays local.

use vessel_proto::ErrorCode;

use super::{Lang, SideWords};
use crate::side_question::Failure;

impl SideWords {
    /// Notice adjacent to the failed side action, without peer error text.
    #[must_use]
    pub const fn failure(&self, failure: Failure) -> &'static str {
        match (self.lang, failure) {
            (Lang::En, Failure::Offline) => "Connect to this machine, then try again.",
            (Lang::Ko, Failure::Offline) => "이 머신에 연결한 뒤 다시 시도하세요.",
            (Lang::En, Failure::Exhausted) => "Reopen the app to continue using side chats.",
            (Lang::Ko, Failure::Exhausted) => "곁대화를 계속 사용하려면 앱을 다시 여세요.",
            (Lang::En, Failure::Busy) => "Other requests are still running. Try again shortly.",
            (Lang::Ko, Failure::Busy) => "다른 요청을 처리하는 중입니다. 잠시 후 다시 시도하세요.",
            (Lang::En, Failure::TimedOut) => {
                "The request did not answer in time. Check the side chat before sending again."
            }
            (Lang::Ko, Failure::TimedOut) => {
                "요청에 대한 응답이 늦어지고 있습니다. 다시 보내기 전에 곁대화를 확인하세요."
            }
            (Lang::En, Failure::InvalidReply) => {
                "The machine returned an unexpected answer. Reconnect and try again."
            }
            (Lang::Ko, Failure::InvalidReply) => {
                "머신에서 예상하지 못한 응답을 받았습니다. 다시 연결한 뒤 시도하세요."
            }
            (Lang::En, Failure::Refused(ErrorCode::Unauthorized)) => {
                "Reconnect to this machine to restore access."
            }
            (Lang::Ko, Failure::Refused(ErrorCode::Unauthorized)) => {
                "이 머신에 다시 연결해 접근 권한을 확인하세요."
            }
            (Lang::En, Failure::Refused(ErrorCode::Forbidden)) => {
                "Side chats need permission to read and operate. Ask the machine owner to grant both."
            }
            (Lang::Ko, Failure::Refused(ErrorCode::Forbidden)) => {
                "곁대화에는 읽기와 조작 권한이 필요합니다. 머신 소유자에게 두 권한을 요청하세요."
            }
            (Lang::En, Failure::Refused(ErrorCode::ProtocolMismatch)) => {
                "Update the app and node so they can connect."
            }
            (Lang::Ko, Failure::Refused(ErrorCode::ProtocolMismatch)) => {
                "앱과 노드를 업데이트한 뒤 연결하세요."
            }
            (Lang::En, Failure::Refused(ErrorCode::NotFound)) => {
                "This conversation is no longer available. Choose another conversation."
            }
            (Lang::Ko, Failure::Refused(ErrorCode::NotFound)) => {
                "이 대화를 더 이상 사용할 수 없습니다. 다른 대화를 선택하세요."
            }
            (Lang::En, Failure::Refused(ErrorCode::Conflict)) => {
                "The side chat changed. Check its current state before trying again."
            }
            (Lang::Ko, Failure::Refused(ErrorCode::Conflict)) => {
                "곁대화가 변경됐습니다. 현재 상태를 확인한 뒤 다시 시도하세요."
            }
            (Lang::En, Failure::Refused(ErrorCode::Unsupported)) => {
                "Side chats are unavailable here. Open an ordinary conversation on an updated node."
            }
            (Lang::Ko, Failure::Refused(ErrorCode::Unsupported)) => {
                "여기서는 곁대화를 사용할 수 없습니다. 업데이트된 노드에서 일반 대화를 여세요."
            }
            (Lang::En, Failure::Refused(ErrorCode::InvalidRequest)) => {
                "The request could not be used. Check the question and the original session's settings before trying again."
            }
            (Lang::Ko, Failure::Refused(ErrorCode::InvalidRequest)) => {
                "요청을 처리할 수 없습니다. 질문과 원래 세션의 설정을 확인한 뒤 다시 시도하세요."
            }
            (Lang::En, Failure::Refused(ErrorCode::HarnessNotInstalled)) => {
                "Install the selected agent on this machine, then try again."
            }
            (Lang::Ko, Failure::Refused(ErrorCode::HarnessNotInstalled)) => {
                "이 머신에 선택한 에이전트를 설치한 뒤 다시 시도하세요."
            }
            (Lang::En, Failure::Refused(ErrorCode::SpawnFailed)) => {
                "The agent could not start. Check its installation and sign-in, then try again."
            }
            (Lang::Ko, Failure::Refused(ErrorCode::SpawnFailed)) => {
                "에이전트를 시작하지 못했습니다. 설치와 로그인 상태를 확인한 뒤 다시 시도하세요."
            }
            (Lang::En, Failure::Refused(ErrorCode::Unavailable)) => {
                "The machine is busy or restarting. Try again shortly."
            }
            (Lang::Ko, Failure::Refused(ErrorCode::Unavailable)) => {
                "머신이 다른 작업을 처리하거나 재시작하는 중입니다. 잠시 후 다시 시도하세요."
            }
            (Lang::En, Failure::Refused(ErrorCode::Internal)) => {
                "The machine could not complete the request. Check its diagnostics before trying again."
            }
            (Lang::Ko, Failure::Refused(ErrorCode::Internal)) => {
                "머신에서 요청을 완료하지 못했습니다. 진단 정보를 확인한 뒤 다시 시도하세요."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_failure_has_a_complete_english_and_korean_next_step() {
        let cases = [
            (
                Failure::Exhausted,
                "Reopen the app to continue using side chats.",
                "곁대화를 계속 사용하려면 앱을 다시 여세요.",
            ),
            (
                Failure::Offline,
                "Connect to this machine, then try again.",
                "이 머신에 연결한 뒤 다시 시도하세요.",
            ),
            (
                Failure::Busy,
                "Other requests are still running. Try again shortly.",
                "다른 요청을 처리하는 중입니다. 잠시 후 다시 시도하세요.",
            ),
            (
                Failure::TimedOut,
                "The request did not answer in time. Check the side chat before sending again.",
                "요청에 대한 응답이 늦어지고 있습니다. 다시 보내기 전에 곁대화를 확인하세요.",
            ),
            (
                Failure::InvalidReply,
                "The machine returned an unexpected answer. Reconnect and try again.",
                "머신에서 예상하지 못한 응답을 받았습니다. 다시 연결한 뒤 시도하세요.",
            ),
            (
                Failure::Refused(ErrorCode::Unauthorized),
                "Reconnect to this machine to restore access.",
                "이 머신에 다시 연결해 접근 권한을 확인하세요.",
            ),
            (
                Failure::Refused(ErrorCode::Forbidden),
                "Side chats need permission to read and operate. Ask the machine owner to grant both.",
                "곁대화에는 읽기와 조작 권한이 필요합니다. 머신 소유자에게 두 권한을 요청하세요.",
            ),
            (
                Failure::Refused(ErrorCode::ProtocolMismatch),
                "Update the app and node so they can connect.",
                "앱과 노드를 업데이트한 뒤 연결하세요.",
            ),
            (
                Failure::Refused(ErrorCode::NotFound),
                "This conversation is no longer available. Choose another conversation.",
                "이 대화를 더 이상 사용할 수 없습니다. 다른 대화를 선택하세요.",
            ),
            (
                Failure::Refused(ErrorCode::Conflict),
                "The side chat changed. Check its current state before trying again.",
                "곁대화가 변경됐습니다. 현재 상태를 확인한 뒤 다시 시도하세요.",
            ),
            (
                Failure::Refused(ErrorCode::Unsupported),
                "Side chats are unavailable here. Open an ordinary conversation on an updated node.",
                "여기서는 곁대화를 사용할 수 없습니다. 업데이트된 노드에서 일반 대화를 여세요.",
            ),
            (
                Failure::Refused(ErrorCode::InvalidRequest),
                "The request could not be used. Check the question and the original session's settings before trying again.",
                "요청을 처리할 수 없습니다. 질문과 원래 세션의 설정을 확인한 뒤 다시 시도하세요.",
            ),
            (
                Failure::Refused(ErrorCode::HarnessNotInstalled),
                "Install the selected agent on this machine, then try again.",
                "이 머신에 선택한 에이전트를 설치한 뒤 다시 시도하세요.",
            ),
            (
                Failure::Refused(ErrorCode::SpawnFailed),
                "The agent could not start. Check its installation and sign-in, then try again.",
                "에이전트를 시작하지 못했습니다. 설치와 로그인 상태를 확인한 뒤 다시 시도하세요.",
            ),
            (
                Failure::Refused(ErrorCode::Unavailable),
                "The machine is busy or restarting. Try again shortly.",
                "머신이 다른 작업을 처리하거나 재시작하는 중입니다. 잠시 후 다시 시도하세요.",
            ),
            (
                Failure::Refused(ErrorCode::Internal),
                "The machine could not complete the request. Check its diagnostics before trying again.",
                "머신에서 요청을 완료하지 못했습니다. 진단 정보를 확인한 뒤 다시 시도하세요.",
            ),
        ];
        for (failure, en, ko) in cases {
            assert_eq!(Lang::En.side_question().failure(failure), en);
            assert_eq!(Lang::Ko.side_question().failure(failure), ko);
        }
    }
}
