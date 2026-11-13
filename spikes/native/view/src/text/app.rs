//! The words of the lead's areas (the sidebar, the board, the archive, the
//! new chat, a session's frame and composer, pairing) and the shared
//! frame, in English and Korean. See `text/mod.rs` for the convention.

use vessel_proto::DisplayStatus;

use super::Lang;
use crate::{Lane, Notice, Phrase, ToolState};

impl Lang {
    /// The words of the lead's areas in this language.
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
    /// A machine's state beside its name in the sidebar, and its row's
    /// detail in the composer's machine list, when it did not answer (the
    /// transport's own text is only the sidebar's tooltip).
    pub unreachable: &'static str,
    /// The line under an unreachable machine's name in the sidebar: what
    /// to do.
    pub unreachable_hint: &'static str,
    /// The session's bar when its machine's connection is lost (once; the
    /// transport's words are only that line's tooltip).
    pub session_lost: &'static str,
    /// What waits on a command the app's link did not take (its queue
    /// full, or stopped) says this, once: nothing reached a machine.
    pub unsent: &'static str,
    pub reload: &'static str,
    // The board.
    pub building: &'static str,
    pub validating: &'static str,
    pub in_review: &'static str,
    pub ready: &'static str,
    // A session.
    pub terminal_not_yet: &'static str,
    pub send: &'static str,
    pub queue: &'static str,
    pub stop: &'static str,
    pub enter_sends: &'static str,
    pub working_queues: &'static str,
    pub ended: &'static str,
    pub resuming: &'static str,
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
    pub key: &'static str,
    pub key_check: &'static str,
    pub details: &'static str,
    pub pair: &'static str,
    pub pairing: &'static str,
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

    /// Starting an ended session again failed.
    #[must_use]
    pub fn resume_failed(&self, why: &str) -> String {
        match self.lang {
            Lang::En => format!("The session could not be started again: {why}"),
            Lang::Ko => format!("세션을 다시 시작하지 못했습니다: {why}"),
        }
    }

    /// The sidebar's foot.
    #[must_use]
    pub fn build(&self, label: &str) -> String {
        format!("Vessel {label}")
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

    /// A session's composer while empty, naming the key that sends
    /// (`key`, Settings' send shortcut as a person reads it).
    #[must_use]
    pub fn composer_placeholder(&self, key: &str) -> String {
        match self.lang {
            Lang::En => format!("Ask the agent… ({key} sends)"),
            Lang::Ko => format!("에이전트에게 요청하세요… (보내기: {key})"),
        }
    }

    /// The new chat's composer while empty and no suggestion is shown,
    /// naming the key that starts.
    #[must_use]
    pub fn prompt_placeholder(&self, key: &str) -> String {
        match self.lang {
            Lang::En => format!("What should the agent do? ({key} starts)"),
            Lang::Ko => format!("에이전트가 할 일을 적으세요 (시작: {key})"),
        }
    }

    /// The status line while a new session's agent starts.
    #[must_use]
    pub fn starting_on(&self, agent: &str, machine: &str) -> String {
        match self.lang {
            Lang::En => format!("Starting {agent} on {machine}…"),
            Lang::Ko => format!("{agent} 시작하는 중… ({machine})"),
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
    unreachable_hint: "Check that the machine is on and its node is running.",
    session_lost: "Not connected to the machine",
    unsent: "Could not send. Try again.",
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
    send: "Send",
    queue: "Queue",
    stop: "Stop",
    enter_sends: "Enter sends.",
    working_queues: "The agent is working; a prompt now waits in the queue.",
    ended: "This session has ended. Sending starts it again with this conversation.",
    resuming: "Starting the session again…",
    a_tool_call: "a tool call",
    machine: "Machine",
    agent: "Agent",
    folder: "Folder",
    folder_placeholder: "A folder on the machine (empty: its home folder)",
    first_prompt: "First prompt",
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
    key: "Key",
    key_check: "This must be the same as the key the machine printed with the link.",
    details: "Details",
    pair: "Pair",
    pairing: "Pairing…",
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
    unreachable: "연결할 수 없음",
    unreachable_hint: "머신이 켜져 있고 노드가 실행 중인지 확인하세요.",
    session_lost: "머신과 연결이 끊겼습니다",
    unsent: "보내지 못했습니다. 다시 시도하세요.",
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
    send: "보내기",
    queue: "대기열에 넣기",
    stop: "멈추기",
    enter_sends: "Enter로 보냅니다.",
    working_queues: "에이전트가 작업 중입니다. 지금 보내면 대기열에서 기다립니다.",
    ended: "끝난 세션입니다. 보내면 이 대화로 다시 시작합니다.",
    resuming: "세션을 다시 시작하는 중…",
    a_tool_call: "도구 호출",
    machine: "머신",
    agent: "에이전트",
    folder: "폴더",
    folder_placeholder: "머신의 폴더 (비우면 홈 폴더)",
    first_prompt: "첫 프롬프트",
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
    key: "키",
    key_check: "머신이 링크와 함께 출력한 키와 같아야 합니다.",
    details: "자세히",
    pair: "연결",
    pairing: "연결하는 중…",
};
