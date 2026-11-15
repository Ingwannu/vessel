//! Sync's whole sentences. Names, paths and commands are separate values.

mod errors;
mod members;
mod rows;
mod times;
use super::Lang;

impl Lang {
    /// Read-only Sync page words.
    #[must_use]
    pub const fn sync(self) -> &'static SyncWords {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

/// One complete language of the Sync status page.
#[derive(Debug)]
pub struct SyncWords {
    /// Language of numeric and interpolated sentences.
    pub lang: Lang,
    /// Overview section title.
    pub overview: &'static str,
    /// Overview explanation; wraps.
    pub lead: &'static str,
    /// Home identity row title.
    pub home: &'static str,
    /// Explanation beneath a proven home; wraps.
    pub home_hint: &'static str,
    /// Normal role explanation when the home identity is unknown; no failure prose.
    pub home_unknown_hint: &'static str,
    /// Short badge of a proven home.
    pub home_badge: &'static str,
    /// Short badge when a comparison is unavailable.
    pub unknown_badge: &'static str,
    /// Shared version explanation.
    pub version_hint: &'static str,
    /// Shared file section title.
    pub files: &'static str,
    /// Empty successful publication; wraps.
    pub empty_files: &'static str,
    /// Names-only tool server section title.
    pub tools: &'static str,
    /// Tool-server row title.
    pub tool: &'static str,
    /// Names-only transport explanation; wraps.
    pub tool_hint: &'static str,
    /// Empty successful server list; wraps.
    pub empty_tools: &'static str,
    /// Member-report section title.
    pub members: &'static str,
    /// Distinguishes last report from a live connection or last sync.
    pub members_hint: &'static str,
    /// Empty successful member list; wraps.
    pub empty_members: &'static str,
    /// Connection section title.
    pub connections: &'static str,
    /// Connection snapshot explanation; wraps.
    pub connections_hint: &'static str,
    /// Empty successful connection list; wraps.
    pub empty_connections: &'static str,
    /// Names-only secret section title.
    pub secrets: &'static str,
    /// Privacy sentence beneath secret row heading; wraps.
    pub secret_hint: &'static str,
    /// Empty successful secret/reference lists; wraps.
    pub empty_secrets: &'static str,
    /// Local-command guidance section title.
    pub commands: &'static str,
    /// Why commands run locally; wraps.
    pub commands_hint: &'static str,
    /// Local-edit notice title; wraps.
    pub changes: &'static str,
    /// Introduction to the reported paths; the copy-location sentence follows them.
    pub changes_hint: &'static str,
    /// Neutral busy badge; short.
    pub loading: &'static str,
    /// Read-only refresh action; short.
    pub refresh: &'static str,
    /// Retry a failed read; short.
    pub retry: &'static str,
    /// No selected machine; wraps.
    pub no_machines: &'static str,
    /// Connection unavailable; wraps.
    pub offline: &'static str,
    /// Refresh row label.
    pub read_controls: &'static str,
    /// Refresh does not force a sync; wraps.
    pub read_hint: &'static str,
}
const EN: SyncWords = SyncWords {
    lang: Lang::En,
    overview: "Shared machine settings",
    lead: "One home shares instructions, skills and tool servers. Members pull changes automatically while their nodes run.",
    home: "Home machine",
    home_hint: "This machine publishes the shared files and tool-server names.",
    home_unknown_hint: "The home provides shared files and tool-server names.",
    home_badge: "Home",
    unknown_badge: "Not checked",
    version_hint: "Increases when the shared files or tool servers change.",
    files: "Shared files",
    empty_files: "No files are shared.",
    tools: "Tool servers",
    tool: "Shared tool server",
    tool_hint: "Only the server name is shown here.",
    empty_tools: "No tool servers are shared.",
    members: "Member reports",
    members_hint: "These are the last member reports available from this machine. Refresh to check again.",
    empty_members: "No member reports yet. Join another machine using the commands below.",
    connections: "Machine connections",
    connections_hint: "Which machines the selected machine can reach, as last reported.",
    empty_connections: "No machine connections were reported.",
    secrets: "Secret names",
    secret_hint: "Only names and presence are shown. Values stay on each machine.",
    empty_secrets: "No secret names were reported.",
    commands: "Manage sync on the machine",
    commands_hint: "Run each command on the machine it affects, using the same data directory as its running node.",
    changes: "Reported local changes",
    changes_hint: "This member reported keeping local changes at these paths:",
    loading: "Loading sync status",
    refresh: "Refresh",
    retry: "Try again",
    no_machines: "No machines yet.",
    offline: "Connect this machine to read its sharing status.",
    read_controls: "Read sharing status again",
    read_hint: "Reads the latest shared version and reports from this machine.",
};
const KO: SyncWords = SyncWords {
    lang: Lang::Ko,
    overview: "머신 간 설정 공유",
    lead: "홈 머신 하나가 지침, 스킬, 도구 서버를 공유합니다. 구성원 머신은 노드가 실행되는 동안 변경 사항을 자동으로 가져옵니다.",
    home: "홈 머신",
    home_hint: "이 머신이 공유 파일과 도구 서버 이름을 제공합니다.",
    home_unknown_hint: "홈 머신이 공유 파일과 도구 서버 이름을 제공합니다.",
    home_badge: "홈",
    unknown_badge: "확인 안 됨",
    version_hint: "공유 파일이나 도구 서버가 바뀌면 버전이 올라갑니다.",
    files: "공유 파일",
    empty_files: "공유하는 파일이 없습니다.",
    tools: "도구 서버",
    tool: "공유 도구 서버",
    tool_hint: "여기에는 서버 이름만 표시합니다.",
    empty_tools: "공유하는 도구 서버가 없습니다.",
    members: "구성원의 보고",
    members_hint: "이 머신에서 확인할 수 있는 마지막 구성원 보고입니다. 새로고침하면 다시 확인합니다.",
    empty_members: "아직 구성원의 보고가 없습니다. 아래 명령으로 다른 머신을 추가하세요.",
    connections: "머신 연결",
    connections_hint: "선택한 머신이 어떤 머신에 연결할 수 있는지 마지막으로 보고한 상태입니다.",
    empty_connections: "보고된 머신 연결이 없습니다.",
    secrets: "비밀값 이름",
    secret_hint: "이름과 저장 여부만 표시합니다. 값은 각 머신에 남습니다.",
    empty_secrets: "보고된 비밀값 이름이 없습니다.",
    commands: "머신에서 동기화 관리",
    commands_hint: "각 명령은 대상 머신에서 실행하세요. 실행 중인 노드와 같은 데이터 디렉터리를 사용해야 합니다.",
    changes: "보고된 로컬 변경",
    changes_hint: "이 머신은 다음 경로의 로컬 변경을 유지했다고 보고했습니다:",
    loading: "동기화 상태 불러오는 중",
    refresh: "새로고침",
    retry: "다시 시도",
    no_machines: "아직 머신이 없습니다.",
    offline: "머신을 연결하면 공유 상태를 확인할 수 있습니다.",
    read_controls: "공유 상태 다시 확인",
    read_hint: "이 머신에서 최신 공유 버전과 보고를 읽습니다.",
};
