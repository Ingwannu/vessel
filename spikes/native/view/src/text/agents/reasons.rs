//! Unknown verdicts explain the reason in full and use the same action
//! label the eventual screen draws; they never name a nonexistent action.

use super::{AgentsWords, Lang};
use vessel_proto::agents::{CheckStep, UnknownReason};

impl AgentsWords {
    /// Installation could not be established.
    #[must_use]
    pub fn install_unknown(&self, reason: &UnknownReason) -> String {
        let why = self.unknown(reason);
        match self.lang {
            Lang::En => format!("Installation is not known. {why}"),
            Lang::Ko => format!("설치 여부를 확인하지 못했습니다. {why}"),
        }
    }

    /// Authentication could not be established.
    #[must_use]
    pub fn auth_unknown(&self, reason: &UnknownReason) -> String {
        let why = self.unknown(reason);
        match self.lang {
            Lang::En => format!("Sign-in is not known. {why}"),
            Lang::Ko => format!("로그인 여부를 확인하지 못했습니다. {why}"),
        }
    }

    fn unknown(&self, reason: &UnknownReason) -> String {
        let check = self.check_status;
        match (self.lang, reason) {
            (Lang::En, UnknownReason::NotInstalled) => {
                "The agent is not installed on this machine.".to_owned()
            }
            (Lang::Ko, UnknownReason::NotInstalled) => {
                "이 머신에 에이전트가 설치되지 않았습니다.".to_owned()
            }
            (Lang::En, UnknownReason::NotChecked) => format!("Select “{check}” to check it."),
            (Lang::Ko, UnknownReason::NotChecked) => format!("“{check}” 버튼을 선택해 확인하세요."),
            (Lang::En, UnknownReason::InUse) => {
                format!("A chat is using this account. Select “{check}” after it ends.")
            }
            (Lang::Ko, UnknownReason::InUse) => format!(
                "이 계정을 사용하는 대화가 있습니다. 대화가 끝난 뒤 “{check}” 버튼을 선택하세요."
            ),
            (Lang::En, UnknownReason::NotSaid) => {
                "The agent did not provide this information during the check.".to_owned()
            }
            (Lang::Ko, UnknownReason::NotSaid) => {
                "에이전트가 확인 과정에서 이 정보를 알려주지 않았습니다.".to_owned()
            }
            (Lang::En, UnknownReason::NoHome) => format!(
                "Set an absolute account-folder path in profiles.toml on this machine, then select “{check}”."
            ),
            (Lang::Ko, UnknownReason::NoHome) => format!(
                "이 머신의 profiles.toml에서 계정 폴더를 절대 경로로 설정한 뒤 “{check}” 버튼을 선택하세요."
            ),
            (Lang::En, UnknownReason::Unread) => format!(
                "The machine could not understand the agent's answer. Update the agent, then select “{check}”."
            ),
            (Lang::Ko, UnknownReason::Unread) => format!(
                "머신이 에이전트의 응답을 해석하지 못했습니다. 에이전트를 업데이트한 뒤 “{check}” 버튼을 선택하세요."
            ),
            (Lang::En, UnknownReason::TimedOut { .. }) => {
                format!("The check took too long. Select “{check}” to try again.")
            }
            (Lang::Ko, UnknownReason::TimedOut { .. }) => {
                format!("확인 시간이 초과되었습니다. “{check}” 버튼을 선택해 다시 시도하세요.")
            }
            (_, UnknownReason::Failed { step, .. }) => self.failed_check(*step),
        }
    }

    fn failed_check(&self, step: CheckStep) -> String {
        let check = self.check_status;
        match (self.lang, step) {
            (Lang::En, CheckStep::Spawn) => format!(
                "The agent could not start. Check that it is installed, then select “{check}”."
            ),
            (Lang::Ko, CheckStep::Spawn) => format!(
                "에이전트를 시작하지 못했습니다. 설치 여부를 확인한 뒤 “{check}” 버튼을 선택하세요."
            ),
            (Lang::En, CheckStep::Initialize) => {
                format!("Could not connect to the agent. Select “{check}” to try again.")
            }
            (Lang::Ko, CheckStep::Initialize) => {
                format!("에이전트에 연결하지 못했습니다. “{check}” 버튼을 선택해 다시 시도하세요.")
            }
            (Lang::En, CheckStep::SessionNew) => format!(
                "The agent could not open a chat for the check. Select “{check}” to try again."
            ),
            (Lang::Ko, CheckStep::SessionNew) => format!(
                "확인을 위한 대화를 열지 못했습니다. “{check}” 버튼을 선택해 다시 시도하세요."
            ),
            (Lang::En, CheckStep::Exited) => {
                format!("The agent closed before answering. Select “{check}” to try again.")
            }
            (Lang::Ko, CheckStep::Exited) => format!(
                "에이전트가 응답하기 전에 종료되었습니다. “{check}” 버튼을 선택해 다시 시도하세요."
            ),
        }
    }
}
