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
};

impl OptionsText {
    /// An agent's option named like one of the composer's own rows, said
    /// with whose it is: "Claude Code agent".
    #[must_use]
    pub fn whose(&self, agent: &str, name: &str) -> String {
        format!("{agent} {name}")
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
