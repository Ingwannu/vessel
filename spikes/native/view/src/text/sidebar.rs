//! The sidebar's words (the page's `features/shell/sidebar*.tsx`): its
//! rows, the machines and status groups, the Display menu and the foot;
//! in English and Korean. See `text/mod.rs` for the convention.

use super::Lang;
use crate::attention::Bucket;
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
    by_project: &'static str,
    by_machine: &'static str,
    by_status: &'static str,
    pub sort: &'static str,
    created: &'static str,
    activity: &'static str,
    pub show_ended: &'static str,
    // A session row's menu.
    pub copy_path: &'static str,
    /// Its toast (Paseo's `sidebar.workspace.toasts.pathCopied`).
    pub path_copied: &'static str,
    /// Its workspace's labels page (Paseo's `workspaceLabels.title`).
    pub labels: &'static str,
    /// The menu's title as a phone's sheet (Paseo's
    /// `sidebar.workspace.actions.menu`, "Workspace actions"; a Vessel row is
    /// a session).
    pub row_actions: &'static str,
    // A row's status (Paseo's `sidebar.statusBucket`), said by a project's
    // badge.
    needs_input: &'static str,
    failed: &'static str,
    ready_to_review: &'static str,
    bucket_working: &'static str,
    done: &'static str,
    // A project's row (Paseo's `sidebar.project`).
    /// Its plus's tooltip (`sidebar.workspace.actions.newWorkspace`).
    pub new_workspace: &'static str,
    /// Its kebab's name and its phone sheet's title (`actions.menu`).
    pub project_actions: &'static str,
    pub open_project_settings: &'static str,
    pub remove_project: &'static str,
    /// The remove row while the node removes it (`actions.removing`).
    pub removing_project: &'static str,
    pub remove_project_title: &'static str,
    pub remove_confirm: &'static str,
    pub cancel: &'static str,
    /// The row ending a project's truncated sessions, and the same row once
    /// they all show (`sidebar.workspace.actions.showMore`, `showLess`).
    pub show_more: &'static str,
    pub show_less: &'static str,
    // A session row's hover card (TRACKER 810, Paseo's
    // `workspace.hoverCard`): its name as a group (the coordinator's ruling
    // for Paseo's leftover "Workspace scripts"), its copy rows, its checks.
    pub session_details: &'static str,
    pub copy_branch_name: &'static str,
    pub checks: &'static str,
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
            Grouping::Project => self.by_project,
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

    /// A status bucket's name (Paseo's `getStatusBucketLabel`).
    #[must_use]
    pub const fn bucket(&self, bucket: Bucket) -> &'static str {
        match bucket {
            Bucket::NeedsInput => self.needs_input,
            Bucket::Failed => self.failed,
            Bucket::Working => self.bucket_working,
            Bucket::Review => self.ready_to_review,
            Bucket::Done => self.done,
        }
    }

    /// The remove question's words (`confirmations.removeMessage`).
    #[must_use]
    pub fn remove_project_body(&self, name: &str) -> String {
        match self.lang {
            Lang::En => {
                format!("Remove \"{name}\" from the sidebar?\n\nFiles on disk will not be changed.")
            }
            Lang::Ko => {
                format!(
                    "\"{name}\"을(를) 사이드바에서 제거할까요?\n\n디스크의 파일은 변경되지 않습니다."
                )
            }
        }
    }

    /// A pull request's badge, said as a link
    /// (`workspace.git.pr.accessibility.pullRequest`).
    #[must_use]
    pub fn pull_request(&self, number: u64) -> String {
        match self.lang {
            Lang::En => format!("Pull request #{number}"),
            Lang::Ko => format!("풀 리퀘스트 #{number}"),
        }
    }

    /// The checks' row, said as a link: its word and their state.
    #[must_use]
    pub fn checks_said(&self, state: &str) -> String {
        format!("{}: {state}", self.checks)
    }

    /// The plus's name (`sidebar.workspace.actions.createWorkspaceFor`).
    #[must_use]
    pub fn create_workspace_for(&self, name: &str) -> String {
        match self.lang {
            Lang::En => format!("Create a new workspace for {name}"),
            Lang::Ko => format!("{name}을(를) 위한 새 워크스페이스 생성"),
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
    by_project: "Project",
    by_machine: "Machine",
    by_status: "Status",
    sort: "Sort",
    created: "Created",
    activity: "Last activity",
    show_ended: "Show ended sessions",
    copy_path: "Copy path",
    path_copied: "Path copied",
    labels: "Labels",
    row_actions: "Session actions",
    needs_input: "Needs input",
    failed: "Failed",
    ready_to_review: "Ready to review",
    bucket_working: "Working",
    done: "Done",
    new_workspace: "New workspace",
    project_actions: "Project actions",
    open_project_settings: "Open project settings",
    remove_project: "Remove project",
    removing_project: "Removing...",
    remove_project_title: "Remove project?",
    remove_confirm: "Remove",
    cancel: "Cancel",
    show_more: "Show more",
    show_less: "Show less",
    session_details: "Session details",
    copy_branch_name: "Copy branch name",
    checks: "Checks",
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
    by_project: "프로젝트",
    by_machine: "머신",
    by_status: "상태",
    sort: "정렬",
    created: "만든 시각",
    activity: "마지막 활동",
    show_ended: "끝난 세션 표시",
    copy_path: "경로 복사",
    path_copied: "경로가 복사되었습니다",
    labels: "라벨",
    row_actions: "세션 작업",
    needs_input: "입력 필요",
    failed: "실패",
    ready_to_review: "검토 대기",
    bucket_working: "실행 중",
    done: "완료",
    new_workspace: "새 워크스페이스",
    project_actions: "프로젝트 작업",
    open_project_settings: "프로젝트 설정 열기",
    remove_project: "프로젝트 제거",
    removing_project: "제거하는 중...",
    remove_project_title: "프로젝트를 제거할까요?",
    remove_confirm: "제거",
    cancel: "취소",
    show_more: "더 보기",
    show_less: "간략히 보기",
    session_details: "세션 정보",
    copy_branch_name: "브랜치 이름 복사",
    checks: "검사",
    settings: "설정",
    back: "뒤로",
    theme: "테마",
    system: "시스템",
    light: "라이트",
    dark: "다크",
};
