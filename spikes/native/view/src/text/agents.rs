//! Agents' sentences. Node diagnostic prose is shown verbatim after a
//! translated sentence; wire enum names are never used as user labels.

use vessel_proto::agents::{CheckStep, InstallState, UnknownReason};

use super::Lang;

/// The Agents page's words in one language.
#[derive(Debug)]
pub struct AgentsWords {
    /// Language of the interpolated sentences.
    pub lang: Lang,
    /// A discovered conversation-capable agent.
    pub ready: &'static str,
    /// Credentials were found, not validated by a model call.
    pub credentials: &'static str,
    /// An explicit negative sign-in verdict.
    pub signed_out: &'static str,
    /// Routed profiles need no agent sign-in.
    pub not_required: &'static str,
}

const EN: AgentsWords = AgentsWords {
    lang: Lang::En,
    ready: "Installed. Can run as a conversation.",
    credentials: "Sign-in credentials are present. Their validity is checked when you send a message.",
    signed_out: "Signed out. Sign in on this machine with the agent's own interface or command.",
    not_required: "Sign-in is not needed for this model route.",
};
const KO: AgentsWords = AgentsWords {
    lang: Lang::Ko,
    ready: "설치되어 있으며 대화로 실행할 수 있습니다.",
    credentials: "로그인 정보가 있습니다. 유효 여부는 메시지를 보낼 때 확인됩니다.",
    signed_out: "로그아웃 상태입니다. 이 머신에서 에이전트의 화면이나 명령으로 로그인하세요.",
    not_required: "이 모델 경로에는 에이전트 로그인이 필요하지 않습니다.",
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
    /// A missing executable, without pretending a runner's package is installed.
    #[must_use]
    pub fn missing(&self, program: &str) -> String {
        match self.lang {
            Lang::En => {
                format!("Not installed. The program {program} was not found on this machine.")
            }
            Lang::Ko => {
                format!("설치되지 않았습니다. 이 머신에서 {program} 프로그램을 찾지 못했습니다.")
            }
        }
    }

    /// A found executable with no conversation support.
    #[must_use]
    pub fn no_conversation(&self, reason: &str) -> String {
        match self.lang {
            Lang::En => format!("Installed. Cannot run as a conversation: {reason}"),
            Lang::Ko => format!("설치되어 있지만 대화로 실행할 수 없습니다: {reason}"),
        }
    }

    /// An ACP mismatch prevents a conversation, even though the program exists.
    #[must_use]
    pub fn incompatible(&self, speaks: u16, needs: u16) -> String {
        match self.lang {
            Lang::En => format!(
                "Installed. Cannot run as a conversation: it speaks ACP {speaks}; Vessel needs ACP {needs}."
            ),
            Lang::Ko => format!(
                "설치되어 있지만 대화로 실행할 수 없습니다. 에이전트는 ACP {speaks}, Vessel은 ACP {needs} 버전입니다."
            ),
        }
    }

    /// The machine could not determine installation.
    #[must_use]
    pub fn install_unknown(&self, reason: &UnknownReason) -> String {
        let why = self.unknown(reason);
        match self.lang {
            Lang::En => format!("Installation is not known. {why}"),
            Lang::Ko => format!("설치 여부를 확인하지 못했습니다. {why}"),
        }
    }

    /// An unknown authentication verdict must never read as signed in.
    #[must_use]
    pub fn auth_unknown(&self, reason: &UnknownReason) -> String {
        let why = self.unknown(reason);
        match self.lang {
            Lang::En => format!("Sign-in is not known. {why}"),
            Lang::Ko => format!("로그인 여부를 확인하지 못했습니다. {why}"),
        }
    }

    fn unknown(&self, reason: &UnknownReason) -> &'static str {
        match (self.lang, reason) {
            (Lang::En, UnknownReason::NotInstalled) => "The agent is not installed.",
            (Lang::Ko, UnknownReason::NotInstalled) => "에이전트가 설치되지 않았습니다.",
            (Lang::En, UnknownReason::NotChecked) => "Ask to check its status.",
            (Lang::Ko, UnknownReason::NotChecked) => "상태 확인을 눌러 확인하세요.",
            (Lang::En, UnknownReason::InUse) => {
                "A session is using this account, so its credentials were left alone."
            }
            (Lang::Ko, UnknownReason::InUse) => {
                "이 계정을 사용하는 세션이 있어 로그인 정보를 건드리지 않았습니다."
            }
            (Lang::En, UnknownReason::NotSaid) => {
                "The agent did not report it during the handshake."
            }
            (Lang::Ko, UnknownReason::NotSaid) => {
                "에이전트가 연결 과정에서 로그인 여부를 알려주지 않았습니다."
            }
            (Lang::En, UnknownReason::NoHome) => {
                "This profile's account folder is not an absolute path."
            }
            (Lang::Ko, UnknownReason::NoHome) => "이 프로필의 계정 폴더가 절대 경로가 아닙니다.",
            (Lang::En, UnknownReason::Unread) => "The machine could not read the check's answer.",
            (Lang::Ko, UnknownReason::Unread) => "머신이 확인 결과를 해석하지 못했습니다.",
            (Lang::En, UnknownReason::TimedOut { .. }) => "The check did not answer in time.",
            (Lang::Ko, UnknownReason::TimedOut { .. }) => "확인 시간이 초과되었습니다.",
            (
                Lang::En,
                UnknownReason::Failed {
                    step: CheckStep::Spawn,
                },
            ) => "The check's program could not start.",
            (
                Lang::Ko,
                UnknownReason::Failed {
                    step: CheckStep::Spawn,
                },
            ) => "확인 프로그램을 시작하지 못했습니다.",
            (
                Lang::En,
                UnknownReason::Failed {
                    step: CheckStep::Initialize,
                },
            ) => "The ACP handshake failed.",
            (
                Lang::Ko,
                UnknownReason::Failed {
                    step: CheckStep::Initialize,
                },
            ) => "ACP 연결에 실패했습니다.",
            (
                Lang::En,
                UnknownReason::Failed {
                    step: CheckStep::SessionNew,
                },
            ) => "The agent could not open a conversation for the check.",
            (
                Lang::Ko,
                UnknownReason::Failed {
                    step: CheckStep::SessionNew,
                },
            ) => "확인을 위한 대화를 열지 못했습니다.",
            (
                Lang::En,
                UnknownReason::Failed {
                    step: CheckStep::Exited,
                },
            ) => "The agent exited before answering the check.",
            (
                Lang::Ko,
                UnknownReason::Failed {
                    step: CheckStep::Exited,
                },
            ) => "에이전트가 확인에 응답하기 전에 종료되었습니다.",
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
            (Lang::En, InstallState::Failed { code: Some(code) }) => {
                format!("Installation failed with exit code {code}.")
            }
            (Lang::Ko, InstallState::Failed { code: Some(code) }) => {
                format!("설치에 실패했습니다. 종료 코드: {code}.")
            }
            (Lang::En, InstallState::Failed { code: None }) => {
                "Installation failed or was stopped.".to_owned()
            }
            (Lang::Ko, InstallState::Failed { code: None }) => {
                "설치에 실패했거나 중지되었습니다.".to_owned()
            }
            (Lang::En, InstallState::TimedOut) => {
                "Installation took too long and was stopped.".to_owned()
            }
            (Lang::Ko, InstallState::TimedOut) => "설치 시간이 초과되어 중지되었습니다.".to_owned(),
        }
    }
}
