//! The words of Settings' machine sections (Machines, Devices, Sync,
//! Agents, Models), in English and Korean (see `text/mod.rs` for the
//! convention). The page's words (`data/machines.ts`, `devices.ts`,
//! `sync.ts`, `agent-status.ts`, `model-rows.ts`).

use vessel_proto::Scope;

use super::Lang;
use crate::ago::Ago;
use crate::machines::{MachineState, NameProblem, Transport};
use crate::sync::MemberState;

impl Lang {
    /// The words of the machine sections in this language.
    #[must_use]
    pub const fn machines(self) -> &'static MachinesText {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

/// One language's sentences.
#[derive(Debug)]
#[expect(missing_docs, reason = "each field is named for the sentence it holds")]
pub struct MachinesText {
    pub lang: Lang,
    // Devices.
    pub never_seen: &'static str,
    // Agents.
    pub available: &'static str,
    pub needs_attention: &'static str,
    pub not_found: &'static str,
    pub no_terminal: &'static str,
    pub on: &'static str,
    pub off: &'static str,
}

const EN: MachinesText = MachinesText {
    lang: Lang::En,
    never_seen: "never seen",
    available: "Available",
    needs_attention: "Needs attention",
    not_found: "Not found",
    no_terminal: "Runs as a conversation; it has no terminal",
    on: "On",
    off: "Off",
};

const KO: MachinesText = MachinesText {
    lang: Lang::Ko,
    never_seen: "접속한 적 없음",
    available: "사용 가능",
    needs_attention: "확인 필요",
    not_found: "찾을 수 없음",
    no_terminal: "대화로 실행되며, 터미널은 없습니다",
    on: "켬",
    off: "끔",
};

impl MachinesText {
    /// How the book reaches a machine.
    #[must_use]
    pub const fn reach(&self, transport: Transport) -> &'static str {
        match (self.lang, transport) {
            (Lang::En, Transport::Direct) => "Direct",
            (Lang::En, Transport::Ssh) => "Over SSH",
            (Lang::En, Transport::Paired) => "Paired",
            (Lang::Ko, Transport::Direct) => "직접",
            (Lang::Ko, Transport::Ssh) => "SSH 경유",
            (Lang::Ko, Transport::Paired) => "페어링",
        }
    }

    /// A machine's state in a word.
    #[must_use]
    pub const fn state(&self, state: MachineState) -> &'static str {
        match (self.lang, state) {
            (Lang::En, MachineState::Connecting) => "Connecting",
            (Lang::En, MachineState::Draining) => "Draining",
            (Lang::En, MachineState::Connected) => "Connected",
            (Lang::En, MachineState::Reconnecting) => "Reconnecting",
            (Lang::En, MachineState::Offline) => "Offline",
            (Lang::Ko, MachineState::Connecting) => "연결 중",
            (Lang::Ko, MachineState::Draining) => "마무리 중",
            (Lang::Ko, MachineState::Connected) => "연결됨",
            (Lang::Ko, MachineState::Reconnecting) => "다시 연결 중",
            (Lang::Ko, MachineState::Offline) => "오프라인",
        }
    }

    /// "1 live session".
    #[must_use]
    pub fn live_count(&self, live: usize) -> String {
        match self.lang {
            Lang::En if live == 1 => "1 live session".to_owned(),
            Lang::En => format!("{live} live sessions"),
            Lang::Ko => format!("실행 중인 세션 {live}개"),
        }
    }

    /// Why a name cannot be a machine's.
    #[must_use]
    pub fn name_problem(&self, problem: &NameProblem) -> String {
        match (self.lang, problem) {
            (Lang::En, NameProblem::Empty) => "Give it a name.".to_owned(),
            (Lang::En, NameProblem::Start) => "Start with a letter or a digit.".to_owned(),
            (Lang::En, NameProblem::Characters) => {
                "Use letters, digits, “.”, “_” and “-” only (no spaces).".to_owned()
            }
            (Lang::En, NameProblem::TooLong) => {
                format!("At most {} characters.", crate::machines::MAX_NAME)
            }
            (Lang::En, NameProblem::Taken(name)) => {
                format!("There is already a machine called {name}.")
            }
            (Lang::Ko, NameProblem::Empty) => "이름을 붙이세요.".to_owned(),
            (Lang::Ko, NameProblem::Start) => "영문자나 숫자로 시작하세요.".to_owned(),
            (Lang::Ko, NameProblem::Characters) => {
                "영문자, 숫자, “.”, “_”, “-”만 쓰세요(공백 없이).".to_owned()
            }
            (Lang::Ko, NameProblem::TooLong) => {
                format!("최대 {}자입니다.", crate::machines::MAX_NAME)
            }
            (Lang::Ko, NameProblem::Taken(name)) => {
                format!("같은 이름의 머신이 이미 있습니다: {name}")
            }
        }
    }

    /// A time before now, short ("3m", "3분").
    #[must_use]
    pub fn ago(&self, ago: Ago) -> String {
        match (self.lang, ago) {
            (Lang::En, Ago::Seconds(n)) => format!("{n}s"),
            (Lang::En, Ago::Minutes(n)) => format!("{n}m"),
            (Lang::En, Ago::Hours(n)) => format!("{n}h"),
            (Lang::En, Ago::Days(n)) => format!("{n}d"),
            (Lang::Ko, Ago::Seconds(n)) => format!("{n}초"),
            (Lang::Ko, Ago::Minutes(n)) => format!("{n}분"),
            (Lang::Ko, Ago::Hours(n)) => format!("{n}시간"),
            (Lang::Ko, Ago::Days(n)) => format!("{n}일"),
        }
    }

    /// What a device's scope lets it do.
    #[must_use]
    pub const fn scope(&self, scope: Scope) -> &'static str {
        match (self.lang, scope) {
            (Lang::En, Scope::Read) => "read",
            (Lang::En, Scope::Operate) => "run sessions",
            (Lang::En, Scope::Admin) => "manage",
            (Lang::En, Scope::Sync) => "sync",
            (Lang::En, Scope::Relay) => "relay",
            (Lang::Ko, Scope::Read) => "읽기",
            (Lang::Ko, Scope::Operate) => "세션 실행",
            (Lang::Ko, Scope::Admin) => "관리",
            (Lang::Ko, Scope::Sync) => "동기화",
            (Lang::Ko, Scope::Relay) => "중계",
        }
    }

    /// "Can read, run sessions".
    #[must_use]
    pub fn may(&self, scopes: &[&str]) -> String {
        let joined = scopes.join(", ");
        match self.lang {
            Lang::En => format!("Can {joined}"),
            Lang::Ko => format!("권한: {joined}"),
        }
    }

    /// "seen 3m ago".
    #[must_use]
    pub fn seen(&self, ago: Ago) -> String {
        let ago = self.ago(ago);
        match self.lang {
            Lang::En => format!("seen {ago} ago"),
            Lang::Ko => format!("{ago} 전 접속"),
        }
    }

    /// "paired 2h ago".
    #[must_use]
    pub fn paired(&self, ago: Ago) -> String {
        let ago = self.ago(ago);
        match self.lang {
            Lang::En => format!("paired {ago} ago"),
            Lang::Ko => format!("{ago} 전 페어링"),
        }
    }

    /// A member's state against the published version.
    #[must_use]
    pub fn member_state(&self, state: MemberState) -> String {
        match (self.lang, state) {
            (Lang::En, MemberState::NotYet) => "Not yet".to_owned(),
            (Lang::En, MemberState::UpToDate) => "Up to date".to_owned(),
            (Lang::En, MemberState::Behind(n)) => format!("{n} behind"),
            (Lang::Ko, MemberState::NotYet) => "아직".to_owned(),
            (Lang::Ko, MemberState::UpToDate) => "최신".to_owned(),
            (Lang::Ko, MemberState::Behind(n)) => format!("{n}개 버전 뒤"),
        }
    }

    /// "Applied version 3".
    #[must_use]
    pub fn applied(&self, applied: Option<u64>) -> String {
        match (self.lang, applied) {
            (Lang::En, None) => "Has not reported".to_owned(),
            (Lang::En, Some(n)) => format!("Applied version {n}"),
            (Lang::Ko, None) => "보고 없음".to_owned(),
            (Lang::Ko, Some(n)) => format!("버전 {n} 적용"),
        }
    }

    /// "1 kept edit".
    #[must_use]
    pub fn kept_edits(&self, edits: usize) -> String {
        match self.lang {
            Lang::En if edits == 0 => "no kept edits".to_owned(),
            Lang::En if edits == 1 => "1 kept edit".to_owned(),
            Lang::En => format!("{edits} kept edits"),
            Lang::Ko if edits == 0 => "유지한 수정 없음".to_owned(),
            Lang::Ko => format!("유지한 수정 {edits}개"),
        }
    }

    /// "reported 1m ago".
    #[must_use]
    pub fn reported(&self, ago: Ago) -> String {
        let ago = self.ago(ago);
        match self.lang {
            Lang::En => format!("reported {ago} ago"),
            Lang::Ko => format!("{ago} 전 보고"),
        }
    }

    /// "Runs in a terminal only: …".
    #[must_use]
    pub fn terminal_only(&self, reason: &str) -> String {
        match self.lang {
            Lang::En => format!("Runs in a terminal only: {reason}"),
            Lang::Ko => format!("터미널에서만 실행됩니다: {reason}"),
        }
    }

    /// Why an agent is not found.
    #[must_use]
    pub fn program_missing(&self, program: Option<&str>) -> String {
        match (self.lang, program) {
            (Lang::En, None) => "Not installed on this machine".to_owned(),
            (Lang::En, Some(program)) => {
                format!("Its program, {program}, is not found on this machine")
            }
            (Lang::Ko, None) => "이 머신에 설치되어 있지 않습니다".to_owned(),
            (Lang::Ko, Some(program)) => {
                format!("이 머신에서 {program} 프로그램을 찾을 수 없습니다")
            }
        }
    }

    /// "Asked 3m ago"; `None` is under a minute.
    #[must_use]
    pub fn asked(&self, ago: Option<Ago>) -> String {
        match (self.lang, ago) {
            (Lang::En, None) => "Asked just now".to_owned(),
            (Lang::En, Some(ago)) => format!("Asked {} ago", self.ago(ago)),
            (Lang::Ko, None) => "방금 확인함".to_owned(),
            (Lang::Ko, Some(ago)) => format!("{} 전 확인함", self.ago(ago)),
        }
    }

    /// "small: qwen/qwen3-coder-flash".
    #[must_use]
    pub fn small_model(&self, model: &str) -> String {
        match self.lang {
            Lang::En => format!("small: {model}"),
            Lang::Ko => format!("소형: {model}"),
        }
    }

    /// Where a profile's models come from.
    #[must_use]
    pub fn models_from(&self, route: Option<&str>) -> String {
        match (self.lang, route) {
            (Lang::En, None) => "the agent's own models".to_owned(),
            (Lang::En, Some(route)) => format!("models from {route}"),
            (Lang::Ko, None) => "에이전트 자체 모델".to_owned(),
            (Lang::Ko, Some(route)) => format!("{route}의 모델"),
        }
    }

    /// "sets 2 variables".
    #[must_use]
    pub fn sets(&self, count: usize) -> String {
        match self.lang {
            Lang::En if count == 1 => "sets 1 variable".to_owned(),
            Lang::En => format!("sets {count} variables"),
            Lang::Ko => format!("변수 {count}개 설정"),
        }
    }
}
