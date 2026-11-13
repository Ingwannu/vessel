//! Every word the app shows, in one table per language (English and
//! Korean), so the language follows the system's and a screen never holds
//! a sentence of its own.
//!
//! [Decision Log]
//! - Purpose: the owner and most of the app's users read Korean (the
//!   coordinator's report on build 0.0.0.0.1): every string goes through
//!   one table that can be switched, from the first screens on.
//! - Alternatives: `rust-i18n` (already in the tree through the component
//!   layer: keys are strings, a missing translation is found at run time);
//!   Fluent (a new dependency and a runtime format).
//! - Choice: a struct with one field per sentence and one constant per
//!   language. A sentence added to one language and not the other does not
//!   compile; a typo in a key cannot happen. Sentences with values in them
//!   are methods, written per language, so word order is the language's.
//! - Cost: a new language is a new constant of every field; plural rules
//!   are written by hand (English needs few here, Korean none).
//! - Not translated yet: the link's errors (they come from `vessel-client`
//!   and the node, in English).

use vessel_proto::DisplayStatus;

use crate::{Lane, Notice, Phrase, ToolState};

/// A language the app speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    /// English.
    En,
    /// Korean.
    Ko,
}

impl Lang {
    /// The language for a BCP 47 tag (`ko-KR`, `en_US.UTF-8`): Korean for
    /// any `ko`, English otherwise.
    #[must_use]
    pub fn from_tag(tag: &str) -> Self {
        let tag = tag.to_ascii_lowercase();
        if tag == "ko" || tag.starts_with("ko-") || tag.starts_with("ko_") || tag.starts_with("ko.")
        {
            Self::Ko
        } else {
            Self::En
        }
    }

    /// The table of this language.
    #[must_use]
    pub const fn text(self) -> &'static Text {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

/// One language's sentences.
#[derive(Debug)]
#[expect(missing_docs, reason = "each field is named for the sentence it holds")]
pub struct Text {
    pub lang: Lang,
    // The sidebar.
    pub app: &'static str,
    pub new_session: &'static str,
    pub board: &'static str,
    pub archive: &'static str,
    pub add_machine: &'static str,
    pub no_machines: &'static str,
    pub online: &'static str,
    pub unreachable: &'static str,
    pub reload: &'static str,
    // The board.
    pub building: &'static str,
    pub validating: &'static str,
    pub in_review: &'static str,
    pub ready: &'static str,
    // A session.
    pub terminal_not_yet: &'static str,
    pub composer_placeholder: &'static str,
    pub send: &'static str,
    pub queue: &'static str,
    pub stop: &'static str,
    pub enter_sends: &'static str,
    pub working_queues: &'static str,
    pub ended: &'static str,
    pub a_tool_call: &'static str,
    // Starting a session.
    pub welcome: &'static str,
    pub suggestions: [&'static str; 4],
    pub home_folder: &'static str,
    pub getting_ready: &'static str,
    pub machine: &'static str,
    pub agent: &'static str,
    pub folder: &'static str,
    pub folder_placeholder: &'static str,
    pub first_prompt: &'static str,
    pub prompt_placeholder: &'static str,
    pub start: &'static str,
    pub starting: &'static str,
    pub no_machine_yet: &'static str,
    pub no_machine_connected: &'static str,
    pub no_agent: &'static str,
    pub choose_first: &'static str,
    // The archive.
    pub nothing_ended: &'static str,
    pub untitled: &'static str,
    // Pairing.
    pub pair_intro: &'static str,
    pub pairing_link: &'static str,
    pub link_placeholder: &'static str,
    pub key: &'static str,
    pub key_check: &'static str,
    pub details: &'static str,
    pub name_label: &'static str,
    pub name_hint: &'static str,
    pub name_is_key: &'static str,
    pub name_is_link: &'static str,
    pub name_empty: &'static str,
    pub name_invalid: &'static str,
    pub name_taken: &'static str,
    pub pair: &'static str,
    pub pairing: &'static str,
    pub paste_first: &'static str,
}

impl Text {
    /// A lane's heading.
    #[must_use]
    pub const fn lane(&self, lane: Lane) -> &'static str {
        match lane {
            Lane::Building => self.building,
            Lane::Validating => self.validating,
            Lane::InReview => self.in_review,
            Lane::Ready => self.ready,
        }
    }

    /// A session's title as shown: its own, or "Untitled session".
    #[must_use]
    pub fn title<'a>(&self, title: &'a str) -> &'a str {
        if title.is_empty() {
            self.untitled
        } else {
            title
        }
    }

    /// The board's heading line.
    #[must_use]
    pub fn board_counts(&self, machines: usize, on_board: usize, archived: usize) -> String {
        match self.lang {
            Lang::En => format!(
                "{machines} {} · {on_board} on the board · {archived} in the archive",
                if machines == 1 { "machine" } else { "machines" }
            ),
            Lang::Ko => format!("머신 {machines}대 · 보드 {on_board}개 · 아카이브 {archived}개"),
        }
    }

    /// The archive's heading line.
    #[must_use]
    pub fn sessions(&self, count: usize) -> String {
        match self.lang {
            Lang::En => format!(
                "{count} {}",
                if count == 1 { "session" } else { "sessions" }
            ),
            Lang::Ko => format!("세션 {count}개"),
        }
    }

    /// The sidebar's foot.
    #[must_use]
    pub fn build(&self, label: &str) -> String {
        format!("Vessel {label} · {}", self.reload)
    }

    /// "just now", "6m ago", "3h ago", "5d ago" (the page's `sinceLabel`).
    #[must_use]
    pub fn since(&self, minutes: i64) -> String {
        let (hours, days) = (minutes.div_euclid(60), minutes.div_euclid(60 * 24));
        match self.lang {
            Lang::En if minutes < 1 => "just now".to_owned(),
            Lang::En if minutes < 60 => format!("{minutes}m ago"),
            Lang::En if hours < 24 => format!("{hours}h ago"),
            Lang::En => format!("{days}d ago"),
            Lang::Ko if minutes < 1 => "방금".to_owned(),
            Lang::Ko if minutes < 60 => format!("{minutes}분 전"),
            Lang::Ko if hours < 24 => format!("{hours}시간 전"),
            Lang::Ko => format!("{days}일 전"),
        }
    }

    /// A card's status line.
    #[must_use]
    pub fn phrase(&self, phrase: &Phrase) -> String {
        match phrase {
            Phrase::Node(status) => self.display_status(*status).to_owned(),
            Phrase::Starting => self.pick("starting", "시작 중"),
            Phrase::Running => self.pick("running", "실행 중"),
            Phrase::Exited(None) => self.pick("exited", "종료됨"),
            Phrase::Exited(Some(code)) => match self.lang {
                Lang::En => format!("exited {code}"),
                Lang::Ko => format!("종료됨 ({code})"),
            },
            Phrase::Failed => self.pick("failed", "실패"),
            Phrase::Interrupted => self.pick("interrupted", "중단됨"),
        }
    }

    fn pick(&self, en: &str, ko: &str) -> String {
        match self.lang {
            Lang::En => en.to_owned(),
            Lang::Ko => ko.to_owned(),
        }
    }

    /// The node's phrase for a card (agent-orchestrator's words in
    /// English, the page's `displayStatusLabel`).
    #[must_use]
    pub const fn display_status(&self, status: DisplayStatus) -> &'static str {
        let (en, ko) = match status {
            DisplayStatus::Working => ("Working", "작업 중"),
            DisplayStatus::Blocked => ("Blocked", "막힘"),
            DisplayStatus::Exited => ("Exited", "종료됨"),
            DisplayStatus::NoSignal => ("No signal", "신호 없음"),
            DisplayStatus::Interrupted => ("Interrupted", "중단됨"),
            DisplayStatus::AwaitingPr => ("Awaiting PR", "PR 대기"),
            DisplayStatus::Idle => ("Idle", "대기"),
            DisplayStatus::FixingCi => ("Fixing CI failures", "CI 실패 수정 중"),
            DisplayStatus::AddressingComments => ("Addressing comments", "코멘트 반영 중"),
            DisplayStatus::NeedsReview => ("Needs review", "리뷰 필요"),
            DisplayStatus::ReviewScheduled => ("Review scheduled", "리뷰 예정"),
            DisplayStatus::Reviewing => ("Reviewing", "리뷰 중"),
            DisplayStatus::ReviewFailed => ("Review failed", "리뷰 실패"),
            DisplayStatus::ReviewPending => ("Review pending", "리뷰 대기"),
            DisplayStatus::Draft => ("Draft", "초안"),
            DisplayStatus::CiFailing => ("CI failing", "CI 실패"),
            DisplayStatus::Commented => ("Commented", "코멘트 있음"),
            DisplayStatus::ChangesRequested => ("Changes requested", "변경 요청됨"),
            DisplayStatus::NeedsHumanReview => ("Needs human review", "사람 리뷰 필요"),
            DisplayStatus::Mergeable => ("Mergeable", "병합 가능"),
            DisplayStatus::Approved => ("Approved", "승인됨"),
            DisplayStatus::Merged => ("Merged", "병합됨"),
            DisplayStatus::Closed => ("Closed without merge", "병합 없이 닫힘"),
            DisplayStatus::Terminated => ("Terminated", "종료됨"),
        };
        match self.lang {
            Lang::En => en,
            Lang::Ko => ko,
        }
    }

    /// A tool call's state word.
    #[must_use]
    pub const fn tool_state(&self, state: ToolState) -> &'static str {
        let (en, ko) = match state {
            ToolState::Pending => ("pending", "대기"),
            ToolState::Running => ("running", "실행 중"),
            ToolState::Done => ("done", "완료"),
            ToolState::Failed => ("failed", "실패"),
        };
        match self.lang {
            Lang::En => en,
            Lang::Ko => ko,
        }
    }

    /// Something that happened in a conversation.
    #[must_use]
    pub fn notice(&self, notice: &Notice) -> String {
        match (self.lang, notice) {
            (Lang::En, Notice::TurnFailed(why)) => format!("The turn failed: {why}"),
            (Lang::Ko, Notice::TurnFailed(why)) => format!("턴이 실패했습니다: {why}"),
            (Lang::En, Notice::Ended) => "The session ended".to_owned(),
            (Lang::Ko, Notice::Ended) => "세션이 끝났습니다".to_owned(),
            (Lang::En, Notice::Failed(why)) => format!("The session failed: {why}"),
            (Lang::Ko, Notice::Failed(why)) => format!("세션이 실패했습니다: {why}"),
            (Lang::En, Notice::Interrupted) => "The session was interrupted".to_owned(),
            (Lang::Ko, Notice::Interrupted) => "세션이 중단되었습니다".to_owned(),
        }
    }

    /// The permission panel's question.
    #[must_use]
    pub fn asks_to_run(&self, what: Option<&str>) -> String {
        let what = what.unwrap_or(self.a_tool_call);
        match self.lang {
            Lang::En => format!("The agent asks to run: {what}"),
            Lang::Ko => format!("에이전트가 실행 허락을 구합니다: {what}"),
        }
    }

    /// The status line while a new session's agent starts.
    #[must_use]
    pub fn starting_on(&self, agent: &str, machine: &str) -> String {
        match self.lang {
            Lang::En => format!("Starting {agent} on {machine}…"),
            Lang::Ko => format!("{machine}에서 {agent} 시작하는 중…"),
        }
    }

    /// Pairing worked.
    #[must_use]
    pub fn paired(&self, name: &str, route: &str) -> String {
        match self.lang {
            Lang::En => format!("Paired with {name} via {route}"),
            Lang::Ko => format!("{name}와(과) 연결했습니다 ({route})"),
        }
    }
}

const EN: Text = Text {
    lang: Lang::En,
    app: "Vessel",
    new_session: "New session",
    board: "Board",
    archive: "Archive",
    add_machine: "Add a machine",
    no_machines: "No machines yet. Add one to begin.",
    online: "online",
    unreachable: "unreachable",
    reload: "reload",
    building: "Building",
    validating: "Validating",
    in_review: "In review",
    ready: "Ready",
    terminal_not_yet: "A terminal session. Terminals are not in this build yet.",
    welcome: "What do you want to work on?",
    suggestions: [
        "Fix a failing test in this project",
        "Explain how this project is structured",
        "Plan the next step for this feature",
        "Find and fix a bug in this project",
    ],
    home_folder: "Home folder",
    getting_ready: "gets ready…",
    composer_placeholder: "Ask the agent… (Enter sends)",
    send: "Send",
    queue: "Queue",
    stop: "Stop",
    enter_sends: "Enter sends.",
    working_queues: "The agent is working; a prompt now waits in the queue.",
    ended: "This session has ended. Resuming it from the app comes next.",
    a_tool_call: "a tool call",
    machine: "Machine",
    agent: "Agent",
    folder: "Folder",
    folder_placeholder: "A folder on the machine (empty: its home folder)",
    first_prompt: "First prompt",
    prompt_placeholder: "What should the agent do? (Enter starts)",
    start: "Start",
    starting: "Starting…",
    no_machine_yet: "No machine yet: add one first.",
    no_machine_connected: "No machine is connected right now.",
    no_agent: "No agent that runs a conversation is installed on this machine.",
    choose_first: "Choose a machine and an agent first.",
    nothing_ended: "Nothing has ended yet.",
    untitled: "Untitled session",
    pair_intro: "Paste the pairing link the machine printed and press Enter. A link works once, for ten minutes.",
    pairing_link: "Pairing link",
    link_placeholder: "vessel://pair#…",
    key: "Key",
    key_check: "This must be the same as the key the machine printed with the link.",
    details: "Details",
    name_label: "Name for this machine",
    name_hint: "A short name you will recognise, for example home-pc.",
    name_is_key: "That is the key. Type a short name instead, for example home-pc.",
    name_is_link: "That is the link; it goes in the field above. Type a short name here, for example home-pc.",
    name_empty: "Type a short name, for example home-pc.",
    name_invalid: "Use letters, digits, '.', '_' and '-', starting with a letter or digit.",
    name_taken: "Another machine already has this name.",
    pair: "Pair",
    pairing: "Pairing…",
    paste_first: "Paste a pairing link first.",
};

const KO: Text = Text {
    lang: Lang::Ko,
    app: "Vessel",
    new_session: "새 세션",
    board: "보드",
    archive: "아카이브",
    add_machine: "머신 추가",
    no_machines: "아직 머신이 없습니다. 머신을 추가해 시작하세요.",
    online: "연결됨",
    unreachable: "연결 안 됨",
    reload: "새로고침",
    building: "작업 중",
    validating: "검증 중",
    in_review: "리뷰 중",
    ready: "준비됨",
    terminal_not_yet: "터미널 세션입니다. 이 빌드에는 아직 터미널이 없습니다.",
    welcome: "어떤 작업을 할까요?",
    suggestions: [
        "이 프로젝트에서 실패하는 테스트를 고쳐 줘",
        "이 프로젝트의 구조를 설명해 줘",
        "이 기능의 다음 단계를 계획해 줘",
        "이 프로젝트에서 버그를 찾아 고쳐 줘",
    ],
    home_folder: "홈 폴더",
    getting_ready: "준비하는 중…",
    composer_placeholder: "에이전트에게 요청하세요… (Enter로 보내기)",
    send: "보내기",
    queue: "대기열에 넣기",
    stop: "멈추기",
    enter_sends: "Enter로 보냅니다.",
    working_queues: "에이전트가 작업 중입니다. 지금 보내면 대기열에서 기다립니다.",
    ended: "끝난 세션입니다. 앱에서 이어 가기는 다음 빌드에 들어갑니다.",
    a_tool_call: "도구 호출",
    machine: "머신",
    agent: "에이전트",
    folder: "폴더",
    folder_placeholder: "머신의 폴더 (비우면 홈 폴더)",
    first_prompt: "첫 프롬프트",
    prompt_placeholder: "에이전트가 할 일을 적으세요 (Enter로 시작)",
    start: "시작",
    starting: "시작하는 중…",
    no_machine_yet: "머신이 없습니다. 먼저 머신을 추가하세요.",
    no_machine_connected: "지금 연결된 머신이 없습니다.",
    no_agent: "이 머신에는 대화를 실행할 에이전트가 설치되어 있지 않습니다.",
    choose_first: "먼저 머신과 에이전트를 고르세요.",
    nothing_ended: "아직 끝난 세션이 없습니다.",
    untitled: "제목 없는 세션",
    pair_intro: "머신이 출력한 페어링 링크를 붙여 넣고 Enter를 누르세요. 링크는 10분 동안 한 번만 쓸 수 있습니다.",
    pairing_link: "페어링 링크",
    link_placeholder: "vessel://pair#…",
    key: "키",
    key_check: "머신이 링크와 함께 출력한 키와 같아야 합니다.",
    details: "자세히",
    name_label: "이 머신의 이름",
    name_hint: "알아보기 쉬운 짧은 이름 (예: home-pc)",
    name_is_key: "이것은 키입니다. 대신 짧은 이름을 적으세요 (예: home-pc).",
    name_is_link: "이것은 링크입니다. 링크는 위 칸에 넣고, 여기에는 짧은 이름을 적으세요 (예: home-pc).",
    name_empty: "짧은 이름을 적으세요 (예: home-pc).",
    name_invalid: "영문자, 숫자, '.', '_', '-'만 쓸 수 있고 영문자나 숫자로 시작해야 합니다.",
    name_taken: "이미 같은 이름의 머신이 있습니다.",
    pair: "연결",
    pairing: "연결하는 중…",
    paste_first: "먼저 페어링 링크를 붙여 넣으세요.",
};
