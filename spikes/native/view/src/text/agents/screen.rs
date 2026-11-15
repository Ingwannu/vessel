//! Agents screen labels, paired at declaration; request failures retain
//! typed sources until the screen chooses a translated boundary message.

use super::{AgentsWords, Lang};
use crate::agent_status::pages::StartProblem;

macro_rules! words {
    ($($name:ident, $context:literal, $en:literal, $ko:literal;)+) => { impl AgentsWords {
        $(#[doc = $context] #[must_use] pub const fn $name(&self) -> &'static str {
            match self.lang { Lang::En => $en, Lang::Ko => $ko }
        })+
    } };
}
words! {
    list_title, "Agents section heading when no machine is configured; keep short. No values.", "Agents", "에이전트";
    row_check_failed, "Agent list summary of a failed installation check; explanation is in details. No values.", "Could not check this agent.", "에이전트를 확인하지 못했습니다.";
    no_machine, "Empty Agents page; instruction when no machine is configured. No values.", "Add a machine to see its agents.", "에이전트를 보려면 머신을 추가하세요.";
    checking, "Progress above the agent details during an explicit status check. No values.", "Checking the agent…", "에이전트를 확인하는 중…";
    refresh, "Agents page refresh button; keep short. No values.", "Refresh", "새로 고침";
    authentication, "Heading above sign-in facts and steps; keep short. No values.", "Sign-in", "로그인";
    launch_command, "Label above a safely quoted Unix launch command; keep short. No values.", "Launch command (sh, bash, zsh)", "실행 명령 (sh·bash·zsh)";
    launch_note, "Below the launch-command label; explains the command’s limited scope. No values.", "Program and arguments only. Account and environment settings stay on the machine.", "프로그램과 인수만 포함합니다. 계정과 환경 설정은 머신에 남습니다.";
    program, "Label above the exact program path or command value; keep short. No values.", "Program", "프로그램";
    details, "Section above installation facts; keep short. No values.", "Agent details", "에이전트 정보";
    status, "Settings-row label for the complete readiness explanation; keep short. No values.", "Status", "상태";
    version, "Label above the reported agent version; keep short. No values.", "Version", "버전";
    version_unknown, "Agent details, in place of an unreported version. No values.", "The agent has not reported a version.", "에이전트가 버전을 알려주지 않았습니다.";
    older_node, "Agents page on an older node with no administration API. No values.", "Update this machine's Vessel to manage agents and check sign-in status.", "에이전트 관리와 로그인 상태 확인을 사용하려면 이 머신의 Vessel을 업데이트하세요.";
    install_warning, "Beside the node-offered command, before the Install button. No values.", "This command runs on the selected machine using the account that runs Vessel there.", "이 명령은 선택한 머신에서 Vessel을 실행하는 계정으로 실행됩니다.";
    install_starting, "Install progress before the node accepts a job. No values.", "Starting the installer…", "설치를 시작하는 중…";
    install_following, "Install progress while reading output from the accepted job. No values.", "Following installation output…", "설치 출력을 읽는 중…";
    install_unavailable, "Install card after an output-read failure; process outcome is unknown. No values.", "The output could no longer be read. The installer may still be running on the machine.", "출력을 더 읽을 수 없습니다. 머신에서 설치가 계속 실행 중일 수 있습니다.";
    install_empty, "Install card when no output was received, including after completion. No values.", "No installer output was received.", "설치 프로그램에서 받은 출력이 없습니다.";
    install_clipped, "Below output after either retention bound was reached. No values.", "The output reached its retention limit. Later output was not kept.", "출력 보관 한도에 도달했습니다. 이후 출력은 보관되지 않았습니다.";
    defaults, "Heading above an agent’s new-chat choices; keep short. No values.", "New chat defaults", "새 대화 기본값";
    defaults_note, "Explains that defaults affect future chats only. No values.", "New chats start with these choices. Existing chats keep their settings.", "새 대화가 이 선택값으로 시작합니다. 기존 대화의 설정은 유지됩니다.";
    no_options, "Defaults card when a probe offers none of the three supported controls. No values.", "This agent offers no model, effort or mode choices.", "이 에이전트는 모델·추론 수준·모드 선택을 제공하지 않습니다.";
    own_defaults, "Defaults reset button; keep short. No values.", "Use the agent's defaults", "에이전트 기본값 사용";
    options_retry, "Button to probe available defaults again; keep short. No values.", "Reload the agent's options", "에이전트 옵션 새로 읽기";
    options_loading, "Defaults card while probing; explains the agent startup. No values.", "Reading the agent's choices. This starts it once; no message is sent.", "에이전트를 한 번 실행해 선택값을 읽습니다. 메시지는 보내지 않습니다.";
    status_unread, "Details when the machine did not return cached status; check action follows. No values.", "The machine has not reported this agent’s status yet.", "머신이 아직 이 에이전트의 상태를 알려주지 않았습니다.";
    error_detail, "Attribution above raw node or transport diagnostics; keep short. No values.", "Details from the machine or connection", "머신 또는 연결의 상세 정보";
    call_failed, "Inline call failure; original diagnostics are retained in a tooltip. No values.", "The machine could not complete this action.", "머신에서 이 작업을 완료하지 못했습니다.";
    disconnected, "Inline operation failure for a disconnected machine. No values.", "The machine is not connected. Connect it, then try again.", "머신에 연결되지 않았습니다. 연결한 뒤 다시 시도하세요.";
    busy, "Inline operation failure when all request slots are occupied. No values.", "Settings is busy. Try again when the current operation finishes.", "설정 작업이 진행 중입니다. 현재 작업이 끝난 뒤 다시 시도하세요.";
    no_transport, "Inline operation failure in the browser without a node transport. No values.", "This browser cannot connect to a machine yet.", "이 브라우저에서는 아직 머신에 연결할 수 없습니다.";
    output_invalid, "Install card after a malformed output cursor; no exit outcome is inferred. No values.", "The machine returned inconsistent installation output. Refresh its connection before trying again.", "머신이 올바르지 않은 설치 출력을 보냈습니다. 연결을 새로 고친 뒤 다시 시도하세요.";
    remove_older_node, "Provider removal refused before dispatch on an old peer. No values.", "Update Vessel on this machine to remove providers.", "프로바이더를 제거하려면 이 머신의 Vessel을 업데이트하세요.";
    remove_permission, "Provider removal refused by the node for lack of administration rights. No values.", "This device cannot remove providers on this machine. Ask the machine’s administrator to grant access.", "이 기기에는 이 머신의 프로바이더를 제거할 권한이 없습니다. 머신 관리자에게 접근 권한을 요청하세요.";
    remove_in_use, "Provider removal refused because an account profile extends it. No values.", "A profile uses this provider. Change or remove that profile on the machine before removing the provider.", "프로필에서 이 프로바이더를 사용 중입니다. 머신에서 해당 프로필을 변경하거나 제거한 뒤 프로바이더를 제거하세요.";
}

impl AgentsWords {
    /// Defaults probe failure above the retry control. The value is that
    /// control's translated label; use the whole sentence as one message.
    #[must_use]
    pub fn options_failed(&self) -> String {
        let reload = self.options_retry();
        match self.lang {
            Lang::En => format!(
                "Could not read the agent's options. Check the agent, then select “{reload}”."
            ),
            Lang::Ko => format!(
                "에이전트의 옵션을 읽지 못했습니다. 에이전트를 확인한 뒤 “{reload}” 버튼을 누르세요."
            ),
        }
    }
    /// Inline failure beside its action; diagnostics follow separately.
    #[must_use]
    pub fn problem(&self, problem: crate::agent_status::problem::Problem) -> String {
        use crate::agent_status::problem::Problem;
        let refresh = self.refresh();
        match problem {
            Problem::NodeChanged => match self.lang {
                Lang::En => format!(
                    "The machine behind this name changed. Select “{refresh}” to read its providers, then confirm again."
                ),
                Lang::Ko => format!(
                    "이 이름이 가리키는 머신이 변경되었습니다. “{refresh}”을 선택해 프로바이더 목록을 읽은 뒤 다시 확인하세요."
                ),
            },
            Problem::RemovalUnknown => match self.lang {
                Lang::En => format!(
                    "The removal result is not known. Select “{refresh}” to read the provider list before trying again."
                ),
                Lang::Ko => format!(
                    "제거 결과를 확인하지 못했습니다. 다시 시도하기 전에 “{refresh}”을 선택해 프로바이더 목록을 읽으세요."
                ),
            },
            Problem::OlderNode => self.remove_older_node().to_owned(),
            Problem::Permission => self.remove_permission().to_owned(),
            Problem::ProviderInUse => self.remove_in_use().to_owned(),
            Problem::Disconnected => self.disconnected().to_owned(),
            Problem::Busy => self.busy().to_owned(),
            Problem::NoTransport => self.no_transport().to_owned(),
            Problem::InvalidOutput => self.output_invalid().to_owned(),
            Problem::Call => self.call_failed().to_owned(),
        }
    }
    /// Label before one exact argv value; ordinal starts at one.
    #[must_use]
    pub fn argument(&self, ordinal: usize) -> String {
        match self.lang {
            Lang::En => format!("Argument {ordinal}"),
            Lang::Ko => format!("인수 {ordinal}"),
        }
    }
    /// A job remains labelled with its actual agent, independently of the open details.
    #[must_use]
    pub fn install_title(&self, name: &str) -> String {
        match self.lang {
            Lang::En => format!("Installation: {name}"),
            Lang::Ko => format!("{name} 설치"),
        }
    }
    /// A refusal before a node call was sent.
    #[must_use]
    pub const fn start_problem(&self, problem: StartProblem) -> &'static str {
        match (self.lang, problem) {
            (Lang::En, StartProblem::Busy) => {
                "Wait for this machine's current operation to finish."
            }
            (Lang::Ko, StartProblem::Busy) => "이 머신의 현재 작업이 끝날 때까지 기다리세요.",
            (Lang::En, StartProblem::Capacity) => {
                "Settings is still working on other machines. Try again shortly."
            }
            (Lang::Ko, StartProblem::Capacity) => {
                "다른 머신의 설정 작업이 진행 중입니다. 잠시 후 다시 시도하세요."
            }
            (Lang::En, StartProblem::Exhausted) => "Close and reopen Vessel, then try again.",
            (Lang::Ko, StartProblem::Exhausted) => "Vessel을 닫고 다시 연 뒤 시도하세요.",
        }
    }
}

#[cfg(test)]
mod redraw_tests {
    use crate::Lang;
    #[test]
    fn options_failure_names_the_actual_retry_control_in_both_languages() {
        for (lang, label, sentence) in [
            (
                Lang::En,
                "Reload the agent's options",
                "Could not read the agent's options. Check the agent, then select “Reload the agent's options”.",
            ),
            (
                Lang::Ko,
                "에이전트 옵션 새로 읽기",
                "에이전트의 옵션을 읽지 못했습니다. 에이전트를 확인한 뒤 “에이전트 옵션 새로 읽기” 버튼을 누르세요.",
            ),
        ] {
            assert_eq!(lang.agents().options_retry(), label);
            assert_eq!(lang.agents().options_failed(), sentence);
        }
    }
}
