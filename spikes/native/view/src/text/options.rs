//! The words of an agent's options on a composer (the agent, model and
//! effort control, and the permissions control), in English and Korean.
//! See `text/mod.rs` for the convention. The page's words, from
//! `web/src/data/options.ts` and `web/src/features/compose/agent-menu.tsx`.

use super::Lang;

impl Lang {
    /// The words of an agent's options in this language.
    #[must_use]
    pub const fn options(self) -> &'static OptionsText {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

/// One language's sentences.
#[derive(Debug)]
#[expect(missing_docs, reason = "each field is named for the sentence it holds")]
pub struct OptionsText {
    pub lang: Lang,
    // The rows the composer names itself (ACP's `mode` is permissions,
    // `thought_level` effort).
    pub agent: &'static str,
    pub model: &'static str,
    pub effort: &'static str,
    pub permissions: &'static str,
    // While the node asks the agent, and when it could not.
    pub loading_models: &'static str,
    pub asking: &'static str,
    pub on: &'static str,
    pub off: &'static str,
    // Paseo's selector words (`i18n/resources/{en,ko}.ts`: `modelSelector`,
    // `agentControls`, `providerSelection`).
    pub select_provider: &'static str,
    pub search_all_models: &'static str,
    pub search_models: &'static str,
    pub default_model: &'static str,
    // The effort card: its fast switch, its reset, and what Claude Code's
    // "Agent" option is (moved to the model's list).
    pub fast_mode: &'static str,
    pub effort_reset: &'static str,
    // The bolt's tooltip (the Codex app's two lines), and the warning over
    // the card's title while the slider is dragged onto the top stop.
    pub fast_speed: &'static str,
    pub fast_usage: &'static str,
    pub maximum_warning: &'static str,
    // The options at the top of the model's list (`extras`), each with what
    // it does; an agent that is not installed.
    pub answering_agent: &'static str,
    pub answering_agent_note: &'static str,
    pub plan_mode: &'static str,
    pub plan_mode_note: &'static str,
    pub not_installed: &'static str,
    pub loading: &'static str,
    pub loading_short: &'static str,
    pub error: &'static str,
    pub unavailable: &'static str,
    pub no_matches: &'static str,
    pub thinking: &'static str,
    pub mode: &'static str,
    pub search_modes: &'static str,
    pub features: &'static str,
    pub setting_on: &'static str,
    pub setting_off: &'static str,
    pub search: &'static str,
    // Paseo's hints on the controls (`agentControls.hints`).
    pub hint_model: &'static str,
    pub hint_thinking: &'static str,
    pub hint_mode: &'static str,
    pub search_hosts: &'static str,
    pub no_hosts: &'static str,
    pub search_folders: &'static str,
    pub no_folders: &'static str,
}

const EN: OptionsText = OptionsText {
    lang: Lang::En,
    agent: "Agent",
    model: "Model",
    effort: "Effort",
    permissions: "Permissions",
    loading_models: "Loading models…",
    asking: "Asking the agent what it offers…",
    on: "On",
    off: "Off",
    select_provider: "Select provider",
    search_all_models: "Search all models...",
    search_models: "Search models...",
    default_model: "Default",
    fast_mode: "Fast mode",
    effort_reset: "Back to the agent's default",
    fast_speed: "1.5x speed",
    fast_usage: "More usage",
    maximum_warning: "Consumes usage limits faster",
    answering_agent: "Answering agent",
    answering_agent_note: "Which of this machine's agent definitions (its agents folder) answers. By default, Claude Code itself.",
    plan_mode: "Plan mode",
    plan_mode_note: "Plans the work and asks before changing anything.",
    not_installed: "Not installed",
    loading: "Loading...",
    loading_short: "Loading",
    error: "Error",
    unavailable: "Unavailable",
    no_matches: "No models match your search",
    thinking: "Thinking",
    mode: "Mode",
    search_modes: "Search modes...",
    features: "Features",
    setting_on: "On",
    setting_off: "Off",
    search: "Search...",
    hint_model: "Change model",
    hint_thinking: "Thinking mode",
    hint_mode: "Change mode",
    search_hosts: "Search machines",
    no_hosts: "No machine is connected",
    search_folders: "Search, or type a folder's path",
    no_folders: "No folder matches",
};

const KO: OptionsText = OptionsText {
    lang: Lang::Ko,
    agent: "에이전트",
    model: "모델",
    effort: "추론 강도",
    permissions: "권한",
    loading_models: "모델 불러오는 중…",
    asking: "에이전트에게 선택지를 묻는 중…",
    on: "켬",
    off: "끔",
    select_provider: "프로바이더 선택",
    search_all_models: "모든 모델 검색...",
    search_models: "모델 검색...",
    default_model: "기본값",
    fast_mode: "빠른 모드",
    effort_reset: "에이전트 기본값으로",
    fast_speed: "1.5배 속도",
    fast_usage: "사용량 증가",
    maximum_warning: "사용 한도를 더 빨리 씁니다",
    answering_agent: "답하는 에이전트",
    answering_agent_note: "이 머신의 에이전트 정의(agents 폴더) 가운데 어느 것이 답할지 고릅니다. 기본으로는 Claude Code 자신이 답합니다.",
    plan_mode: "계획 모드",
    plan_mode_note: "할 일을 먼저 계획하고, 무엇이든 바꾸기 전에 묻습니다.",
    not_installed: "설치되지 않음",
    loading: "불러오는 중...",
    loading_short: "불러오는 중",
    error: "오류",
    unavailable: "사용할 수 없음",
    no_matches: "검색과 일치하는 모델이 없습니다",
    thinking: "사고",
    mode: "모드",
    search_modes: "모드 검색...",
    features: "기능",
    setting_on: "켜짐",
    setting_off: "꺼짐",
    search: "검색...",
    hint_model: "모델 변경",
    hint_thinking: "사고 모드",
    hint_mode: "모드 변경",
    search_hosts: "머신 검색",
    no_hosts: "연결된 머신이 없습니다",
    search_folders: "폴더 검색 또는 경로 입력",
    no_folders: "일치하는 폴더가 없습니다",
};

impl OptionsText {
    /// An agent's option named like one of the composer's own rows, said
    /// with whose it is: "Claude Code agent".
    #[must_use]
    pub fn whose(&self, agent: &str, name: &str) -> String {
        format!("{agent} {name}")
    }

    /// A provider's count of models: "3 models".
    #[must_use]
    pub fn model_count(&self, count: usize) -> String {
        match (self.lang, count) {
            (Lang::En, 1) => "1 model".to_owned(),
            (Lang::En, _) => format!("{count} models"),
            (Lang::Ko, _) => format!("모델 {count}개"),
        }
    }

    /// The row that uses a typed folder: "Use ~/work".
    #[must_use]
    pub fn use_folder(&self, path: &str) -> String {
        match self.lang {
            Lang::En => format!("Use {path}"),
            Lang::Ko => format!("{path} 사용"),
        }
    }

    /// The agent's other settings' heading: "Claude Code settings".
    #[must_use]
    pub fn settings(&self, agent: &str) -> String {
        match self.lang {
            Lang::En => format!("{agent} settings"),
            Lang::Ko => format!("{agent} 설정"),
        }
    }

    /// The agent did not say what it offers, and why.
    #[must_use]
    pub fn failed(&self, why: &str) -> String {
        match self.lang {
            Lang::En => format!(
                "The agent did not say what it offers: {why} It starts on its own defaults."
            ),
            Lang::Ko => format!(
                "에이전트가 선택지를 알려 주지 않았습니다: {why} 에이전트의 기본값으로 시작합니다."
            ),
        }
    }
}
