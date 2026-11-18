//! The sidebar's words (the page's `features/shell/sidebar*.tsx`): its
//! rows, the machines and status groups, the Display menu and the foot;
//! in English and Korean. See `text/mod.rs` for the convention.

use super::Lang;
use crate::prefs::{Grouping, SessionSort};
use crate::sidebar::SessionMark;

impl Lang {
    /// The sidebar's words in this language.
    #[must_use]
    pub const fn sidebar(self) -> &'static SidebarWords {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

/// One language's sentences.
#[derive(Debug)]
#[expect(missing_docs, reason = "each field is named for the sentence it holds")]
pub struct SidebarWords {
    pub lang: Lang,
    // The rows at the top.
    pub search: &'static str,
    pub new_session: &'static str,
    pub board: &'static str,
    // The lists.
    pub machines: &'static str,
    pub sessions: &'static str,
    pub add_machine: &'static str,
    pub no_sessions: &'static str,
    pub connected: &'static str,
    pub offline: &'static str,
    pub unread: &'static str,
    needs_you: &'static str,
    working: &'static str,
    idle: &'static str,
    ended: &'static str,
    // The Display menu.
    pub display: &'static str,
    pub grouping: &'static str,
    by_machine: &'static str,
    by_status: &'static str,
    pub sort: &'static str,
    created: &'static str,
    activity: &'static str,
    pub show_ended: &'static str,
    // A session row's menu.
    pub copy_path: &'static str,
    /// Its workspace's labels page (Paseo's `workspaceLabels.title`).
    pub labels: &'static str,
    /// The menu's title as a phone's sheet (Paseo's
    /// `sidebar.workspace.actions.menu`, "Workspace actions"; a Vessel row is
    /// a session).
    pub row_actions: &'static str,
    // The foot.
    pub settings: &'static str,
    pub back: &'static str,
    pub theme: &'static str,
    pub system: &'static str,
    pub light: &'static str,
    pub dark: &'static str,
}

impl SidebarWords {
    /// A status group's heading, and its mark's name.
    #[must_use]
    pub const fn mark(&self, mark: SessionMark) -> &'static str {
        match mark {
            SessionMark::NeedsYou => self.needs_you,
            SessionMark::Working => self.working,
            SessionMark::Idle => self.idle,
            SessionMark::Ended => self.ended,
        }
    }

    /// A grouping's name in the Display menu.
    #[must_use]
    pub const fn group(&self, group: Grouping) -> &'static str {
        match group {
            Grouping::Machine => self.by_machine,
            Grouping::Status => self.by_status,
        }
    }

    /// An order's name in the Display menu.
    #[must_use]
    pub const fn sort_by(&self, sort: SessionSort) -> &'static str {
        match sort {
            SessionSort::Created => self.created,
            SessionSort::Activity => self.activity,
        }
    }

    /// The row under a list that opens the rest on the board.
    #[must_use]
    pub fn more(&self, count: usize) -> String {
        match self.lang {
            Lang::En => format!("{count} more"),
            Lang::Ko => format!("{count}개 더"),
        }
    }
}

const EN: SidebarWords = SidebarWords {
    lang: Lang::En,
    search: "Search",
    new_session: "New session",
    board: "Board",
    machines: "Machines",
    sessions: "Sessions",
    add_machine: "Add machine",
    no_sessions: "No sessions",
    connected: "Connected",
    offline: "Offline",
    unread: "unread",
    needs_you: "Needs you",
    working: "Working",
    idle: "Idle",
    ended: "Ended",
    display: "Display",
    grouping: "Grouping",
    by_machine: "Machine",
    by_status: "Status",
    sort: "Sort",
    created: "Created",
    activity: "Last activity",
    show_ended: "Show ended sessions",
    copy_path: "Copy path",
    labels: "Labels",
    row_actions: "Session actions",
    settings: "Settings",
    back: "Back",
    theme: "Theme",
    system: "System",
    light: "Light",
    dark: "Dark",
};

const KO: SidebarWords = SidebarWords {
    lang: Lang::Ko,
    search: "검색",
    new_session: "새 세션",
    board: "보드",
    machines: "머신",
    sessions: "세션",
    add_machine: "머신 추가",
    no_sessions: "세션 없음",
    connected: "연결됨",
    offline: "오프라인",
    unread: "읽지 않음",
    needs_you: "확인 필요",
    working: "진행 중",
    idle: "대기 중",
    ended: "끝남",
    display: "보기",
    grouping: "묶기",
    by_machine: "머신",
    by_status: "상태",
    sort: "정렬",
    created: "만든 시각",
    activity: "마지막 활동",
    show_ended: "끝난 세션 표시",
    copy_path: "경로 복사",
    labels: "라벨",
    row_actions: "세션 작업",
    settings: "설정",
    back: "뒤로",
    theme: "테마",
    system: "시스템",
    light: "라이트",
    dark: "다크",
};
