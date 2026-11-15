//! Agents' words in both languages. Node prose is labelled separately,
//! never interpolated into a translated instruction.

use super::Lang;
use crate::agent_status::readiness::AgentBadge;
use vessel_proto::agents::InstallState;

mod reasons;
mod row;
mod screen;

/// Agents' sentences and action labels in one language.
#[derive(Debug)]
pub struct AgentsWords {
    /// Language of interpolated sentences.
    pub lang: Lang,
    /// Installed. Can chat.
    pub ready: &'static str,
    /// Installed, but it cannot chat on this machine.
    pub cannot_chat: &'static str,
    /// Not installed on this machine.
    pub not_installed: &'static str,
    /// This agent is older than this version of Vessel supports. Update the agent.
    pub too_old: &'static str,
    /// This agent is newer than this version of Vessel supports. Update Vessel.
    pub too_new: &'static str,
    /// Sign-in details are saved. They will be checked when you send a message.
    pub credentials: &'static str,
    /// Signed out. Follow the sign-in steps below.
    pub signed_out: &'static str,
    /// No sign-in needed. This profile uses a model provider configured on this machine.
    pub not_required: &'static str,
    /// Check status
    pub check_status: &'static str,
    /// Install adapter
    pub install_adapter: &'static str,
    /// The node-offered program installer control.
    pub install: &'static str,
    /// Message from machine
    pub machine_message: &'static str,
    /// Account folder
    pub account_folder: &'static str,
    /// Use this profile's account folder, selected by the variable below. Its value stays on the machine.
    pub account_instruction: &'static str,
    /// Sign-in command
    pub sign_in_command: &'static str,
    /// Run this command on the selected machine, using this profile's account.
    pub command_instruction: &'static str,
    /// Sign-in method from machine
    pub sign_in_method: &'static str,
    /// Open the agent on this machine and choose the sign-in method shown below.
    pub method_instruction: &'static str,
    /// Open the agent
    pub open_agent: &'static str,
    /// Open the agent on this machine and sign in there.
    pub open_instruction: &'static str,
}

const EN: AgentsWords = AgentsWords {
    lang: Lang::En,
    ready: "Installed. Can chat.",
    cannot_chat: "Installed, but it cannot chat on this machine.",
    not_installed: "Not installed on this machine.",
    too_old: "This agent is older than this version of Vessel supports. Update the agent.",
    too_new: "This agent is newer than this version of Vessel supports. Update Vessel.",
    credentials: "Sign-in details are saved. They will be checked when you send a message.",
    signed_out: "Signed out. Follow the sign-in steps below.",
    not_required: "No sign-in needed. This profile uses a model provider configured on this machine.",
    check_status: "Check status",
    install_adapter: "Install adapter",
    install: "Install",
    machine_message: "Message from machine",
    account_folder: "Account folder",
    account_instruction: "Use this profile's account folder, selected by the variable below. Its value stays on the machine.",
    sign_in_command: "Sign-in command",
    command_instruction: "Run this command on the selected machine, using this profile's account.",
    sign_in_method: "Sign-in method from machine",
    method_instruction: "Open the agent on this machine and choose the sign-in method shown below.",
    open_agent: "Open the agent",
    open_instruction: "Open the agent on this machine and sign in there.",
};
const KO: AgentsWords = AgentsWords {
    lang: Lang::Ko,
    ready: "설치되어 있으며 대화할 수 있습니다.",
    cannot_chat: "설치되어 있지만 이 머신에서는 대화할 수 없습니다.",
    not_installed: "이 머신에 설치되지 않았습니다.",
    too_old: "이 에이전트는 현재 Vessel이 지원하는 버전보다 오래되었습니다. 에이전트를 업데이트하세요.",
    too_new: "이 에이전트는 현재 Vessel이 지원하는 버전보다 최신입니다. Vessel을 업데이트하세요.",
    credentials: "로그인 정보가 저장되어 있습니다. 메시지를 보낼 때 유효 여부를 확인합니다.",
    signed_out: "로그아웃 상태입니다. 아래 단계에 따라 로그인하세요.",
    not_required: "로그인이 필요하지 않습니다. 이 프로필은 머신에 설정된 모델 공급자를 사용합니다.",
    check_status: "상태 확인",
    install_adapter: "어댑터 설치",
    install: "설치",
    machine_message: "머신 메시지",
    account_folder: "계정 폴더",
    account_instruction: "아래 변수가 선택하는 프로필의 계정 폴더를 사용하세요. 변수 값은 머신에만 보관됩니다.",
    sign_in_command: "로그인 명령",
    command_instruction: "선택한 머신에서 이 프로필의 계정으로 아래 명령을 실행하세요.",
    sign_in_method: "머신이 알려준 로그인 방법",
    method_instruction: "이 머신에서 에이전트를 열고 아래 로그인 방법을 선택하세요.",
    open_agent: "에이전트 열기",
    open_instruction: "이 머신에서 에이전트를 열고 로그인하세요.",
};

impl Lang {
    /// Agents' sentences in this language.
    #[must_use]
    pub const fn agents(self) -> &'static AgentsWords {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

impl AgentsWords {
    /// Name the actual program installer only when that action is offered.
    #[must_use]
    pub fn program_missing(&self) -> String {
        let label = self.install;
        match self.lang {
            Lang::En => format!("Not installed on this machine. Select “{label}” below."),
            Lang::Ko => {
                format!("이 머신에 설치되지 않았습니다. 아래의 “{label}” 버튼을 선택하세요.")
            }
        }
    }
    /// Updating an older agent is followed by the actual named check action.
    #[must_use]
    pub fn older_agent(&self) -> String {
        let check = self.check_status;
        match self.lang {
            Lang::En => format!(
                "This agent is older than this version of Vessel supports. Update the agent. Select “{check}” to check again."
            ),
            Lang::Ko => format!(
                "이 에이전트는 현재 Vessel이 지원하는 버전보다 오래되었습니다. 에이전트를 업데이트하세요. “{check}” 버튼을 선택해 다시 확인하세요."
            ),
        }
    }

    /// Short state labels after Paseo's provider rows; no disabled state
    /// exists because the node has no enable switch.
    #[must_use]
    pub const fn badge(&self, badge: AgentBadge) -> &'static str {
        match (self.lang, badge) {
            (Lang::En, AgentBadge::Available) => "Available",
            (Lang::Ko, AgentBadge::Available) => "사용 가능",
            (Lang::En, AgentBadge::NotInstalled) => "Not installed",
            (Lang::Ko, AgentBadge::NotInstalled) => "미설치",
            (Lang::En, AgentBadge::NeedsAttention) => "Needs attention",
            (Lang::Ko, AgentBadge::NeedsAttention) => "확인 필요",
            (Lang::En, AgentBadge::Error) => "Error",
            (Lang::Ko, AgentBadge::Error) => "오류",
            (Lang::En, AgentBadge::Unknown) => "Not known",
            (Lang::Ko, AgentBadge::Unknown) => "확인 안 됨",
        }
    }

    /// Name the actual install control, only when that action is offered.
    #[must_use]
    pub fn adapter_missing(&self) -> String {
        let label = self.install_adapter;
        match self.lang {
            Lang::En => {
                format!("Installed, but its chat adapter is missing. Select “{label}” below.")
            }
            Lang::Ko => format!(
                "설치되어 있지만 대화에 필요한 어댑터가 없습니다. 아래의 “{label}” 버튼을 선택하세요."
            ),
        }
    }
    /// Installation output's terminal state in ordinary words.
    #[must_use]
    pub fn install_state(&self, state: InstallState) -> String {
        match (self.lang, state) {
            (Lang::En, InstallState::Running) => "Installing…".to_owned(),
            (Lang::Ko, InstallState::Running) => "설치 중…".to_owned(),
            (Lang::En, InstallState::Succeeded) => "Installation finished.".to_owned(),
            (Lang::Ko, InstallState::Succeeded) => "설치가 완료되었습니다.".to_owned(),
            (
                Lang::En,
                InstallState::Failed {
                    code: Some(code), ..
                },
            ) => {
                format!("Installation failed with exit code {code}.")
            }
            (
                Lang::Ko,
                InstallState::Failed {
                    code: Some(code), ..
                },
            ) => {
                format!("설치에 실패했습니다. 종료 코드: {code}.")
            }
            (Lang::En, InstallState::Failed { code: None, .. }) => {
                "Installation failed or was stopped.".to_owned()
            }
            (Lang::Ko, InstallState::Failed { code: None, .. }) => {
                "설치에 실패했거나 중지되었습니다.".to_owned()
            }
            (Lang::En, InstallState::TimedOut) => {
                "Installation took too long and was stopped.".to_owned()
            }
            (Lang::Ko, InstallState::TimedOut) => "설치 시간이 초과되어 중지되었습니다.".to_owned(),
        }
    }
}
