//! The words of Settings' frame and of its General, Appearance and About
//! sections, in English and Korean (see `text/mod.rs` for the
//! convention). The page's words (`features/settings/*.tsx`), with "this
//! page" made "Vessel" where the app is meant.

use super::Lang;
use crate::prefs::{ContentWidth, Kept, LaunchOnly, Theme, TurnDelivery};
use crate::settings::{Group, Section};

impl Lang {
    /// The words of Settings in this language.
    #[must_use]
    pub const fn settings(self) -> &'static SettingsText {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

/// One language's sentences.
#[derive(Debug)]
#[expect(missing_docs, reason = "each field is named for the sentence it holds")]
pub struct SettingsText {
    pub lang: Lang,
    // The frame.
    pub settings: &'static str,
    pub back: &'static str,
    pub machine: &'static str,
    pub no_machines: &'static str,
    pub switch_machine: &'static str,
    // General.
    pub general_lead: &'static str,
    pub sending: &'static str,
    pub send_shortcut: &'static str,
    pub default_send: &'static str,
    pub queue: &'static str,
    pub steer: &'static str,
    pub interrupt: &'static str,
    pub kept: &'static str,
    pub preferences: &'static str,
    pub kept_note: &'static str,
    pub this_launch: &'static str,
    pub path_copied: &'static str,
    pub refused: &'static str,
    // Appearance.
    pub appearance_lead: &'static str,
    pub theme: &'static str,
    pub colour_scheme: &'static str,
    pub colour_scheme_note: &'static str,
    pub system: &'static str,
    pub light: &'static str,
    pub dark: &'static str,
    pub zinc: &'static str,
    pub midnight: &'static str,
    pub claude: &'static str,
    pub ghostty: &'static str,
    pub pure_black: &'static str,
    pub fonts: &'static str,
    pub interface_size: &'static str,
    pub interface_size_note: &'static str,
    pub code_size: &'static str,
    pub code_size_note: &'static str,
    pub layout: &'static str,
    pub content_width: &'static str,
    pub content_width_note: &'static str,
    pub comfortable: &'static str,
    pub wide: &'static str,
    pub full: &'static str,
    // About.
    pub about_lead: &'static str,
    pub version: &'static str,
    pub licence: &'static str,
    pub licences: &'static str,
    pub connected_machines: &'static str,
    pub offline: &'static str,
    pub agents_unavailable: &'static str,
    pub idle_release: &'static str,
    pub idle_release_info: &'static str,
    pub idle_too_old: &'static str,
    pub idle_forbidden: &'static str,
    pub idle_refused: &'static str,
}

const EN: SettingsText = SettingsText {
    lang: Lang::En,
    settings: "Settings",
    back: "Back",
    machine: "Machine",
    no_machines: "No machines yet.",
    switch_machine: "Switch machine",
    general_lead: "How the composer sends, and where Vessel keeps what you choose.",
    sending: "Sending",
    send_shortcut: "Send shortcut",
    default_send: "Default send",
    queue: "Queue",
    steer: "Steer",
    interrupt: "Interrupt",
    kept: "Kept",
    preferences: "Preferences",
    kept_note: "Kept by Vessel on this computer, so every launch opens with them.",
    this_launch: "This launch only",
    path_copied: "The path was copied.",
    refused: "That change was not kept",
    appearance_lead: "How Vessel looks on this computer.",
    theme: "Theme",
    colour_scheme: "Colour scheme",
    colour_scheme_note: "Follow the system, or stay light or dark.",
    system: "System",
    light: "Light",
    dark: "Dark",
    zinc: "Zinc",
    midnight: "Midnight",
    claude: "Claude",
    ghostty: "Ghostty",
    pure_black: "Pure black",
    fonts: "Fonts",
    interface_size: "Interface size",
    interface_size_note: "Used for navigation, controls, and labels",
    code_size: "Code size",
    code_size_note: "Used for code, diffs, and terminal output",
    layout: "Layout",
    content_width: "Content width",
    content_width_note: "Max width of chat and Markdown files on wide screens",
    comfortable: "Comfortable",
    wide: "Wide",
    full: "Full",
    about_lead: "This app's version and licence, and each machine's.",
    version: "Version",
    licence: "Licence",
    licences: "Licences",
    // About's section of each machine's node version (Paseo's
    // `settings.about.connectedHosts`, with Vessel's word for a host) and
    // the value shown for one that did not answer. Short.
    connected_machines: "Connected machines",
    offline: "Offline",
    // Settings › Agents (Paseo's `settings.host.agents`): the card that
    // stands for the section while the machine is not connected; the
    // idle-release row's title, its tooltip (two paragraphs), and what it
    // says when the node cannot take the change. "Machine" where Paseo
    // says "host".
    agents_unavailable: "Connect to this machine to manage agents",
    idle_release: "Release idle agents",
    idle_release_info: "Stops the agent's process while keeping its conversation. Send a message to resume.\n\nActive work, pending notifications, and provider safety checks prevent cleanup.",
    idle_too_old: "This machine's node is too old to change this from the app. Update the node to change it here.",
    idle_forbidden: "This device may not change this machine's settings.",
    idle_refused: "The machine did not take the change. Try again.",
};

const KO: SettingsText = SettingsText {
    lang: Lang::Ko,
    settings: "설정",
    back: "뒤로",
    machine: "머신",
    no_machines: "아직 머신이 없습니다.",
    switch_machine: "머신 바꾸기",
    general_lead: "작성창이 보내는 방식과, 고른 설정을 Vessel이 보관하는 곳.",
    sending: "전송",
    send_shortcut: "전송 단축키",
    default_send: "기본 전송",
    queue: "대기열",
    steer: "지시",
    interrupt: "중단",
    kept: "보관",
    preferences: "환경설정",
    kept_note: "Vessel이 이 컴퓨터에 보관하므로, 실행할 때마다 그대로 열립니다.",
    this_launch: "이번 실행에만",
    path_copied: "경로를 복사했습니다.",
    refused: "그 변경은 보관되지 않았습니다",
    appearance_lead: "이 컴퓨터에서 Vessel이 보이는 방식.",
    theme: "테마",
    colour_scheme: "색 구성",
    colour_scheme_note: "시스템을 따르거나, 밝게 또는 어둡게 고정합니다.",
    system: "시스템",
    light: "밝게",
    dark: "어둡게",
    zinc: "Zinc",
    midnight: "Midnight",
    claude: "Claude",
    ghostty: "Ghostty",
    pure_black: "순수 검정",
    fonts: "글꼴",
    interface_size: "인터페이스 크기",
    interface_size_note: "탐색, 컨트롤 및 레이블에 사용됩니다",
    code_size: "코드 크기",
    code_size_note: "코드, diff 및 터미널 출력에 사용됩니다",
    layout: "레이아웃",
    content_width: "콘텐츠 너비",
    content_width_note: "와이드 화면에서 채팅과 Markdown 파일의 최대 너비",
    comfortable: "편안하게",
    wide: "넓게",
    full: "전체",
    about_lead: "이 앱과 각 머신의 버전과 라이선스.",
    version: "버전",
    licence: "라이선스",
    licences: "라이선스",
    connected_machines: "연결된 머신",
    offline: "오프라인",
    agents_unavailable: "에이전트를 관리하려면 이 머신에 연결하세요",
    idle_release: "유휴 에이전트 해제",
    idle_release_info: "대화를 유지하면서 에이전트 프로세스를 중지합니다. 메시지를 보내면 다시 이어집니다.\n\n진행 중인 작업, 대기 중인 알림, 프로바이더 안전 검사가 있으면 정리하지 않습니다.",
    idle_too_old: "이 머신의 노드가 오래되어 앱에서 바꿀 수 없습니다. 노드를 업데이트하면 여기서 바꿀 수 있습니다.",
    idle_forbidden: "이 기기에는 이 머신의 설정을 바꿀 권한이 없습니다.",
    idle_refused: "머신이 변경을 받지 않았습니다. 다시 시도하세요.",
};

impl SettingsText {
    /// The idle-release row's hint (Paseo's `idleRuntimeCleanup.hint`)
    /// for the minutes in force: Paseo's hour as Paseo says it, other
    /// minutes as a whole sentence with the count; `None` (off) as the
    /// hour, Paseo's default.
    #[must_use]
    pub fn idle_hint(&self, minutes: Option<u16>) -> String {
        match (self.lang, minutes) {
            (Lang::En, None | Some(60)) => "Free up memory after an hour of inactivity.".to_owned(),
            (Lang::En, Some(1)) => "Free up memory after a minute of inactivity.".to_owned(),
            (Lang::En, Some(minutes)) => {
                format!("Free up memory after {minutes} minutes of inactivity.")
            }
            (Lang::Ko, None | Some(60)) => {
                "1시간 동안 활동이 없으면 메모리를 확보합니다.".to_owned()
            }
            (Lang::Ko, Some(minutes)) => {
                format!("{minutes}분 동안 활동이 없으면 메모리를 확보합니다.")
            }
        }
    }

    /// A section's name in the list and its page's title: Paseo's
    /// (`en.ts` `settings.sections`, `settings.hostSections`, and
    /// `openProject.tiles.pairDevice.title` for "Pair device"; `ko.ts` the
    /// same keys). Models and Sync are Vessel's own. Short: one line of
    /// the list. Korean "터미널" names both Terminal (the app's) and
    /// Terminals (the machine's), as Paseo's Korean does; they are under
    /// different headings.
    #[must_use]
    pub fn section(
        &self,
        section: Section,
        catalog: &'static crate::localization::Translator,
    ) -> &'static str {
        match (self.lang, section) {
            (Lang::En, Section::General) => "General",
            (Lang::En, Section::Appearance) => "Appearance",
            (Lang::En, Section::Sidebar) => "Sidebar",
            (_, Section::Chat) => catalog.chat().title(),
            (Lang::En, Section::Terminal) => "Terminal",
            (Lang::En, Section::Browser) => "Browser",
            (Lang::En, Section::Editor) => "Editor",
            (Lang::En, Section::Files) => "Files",
            (Lang::En, Section::Shortcuts) => "Shortcuts",
            (Lang::En, Section::Integrations) => "Integrations",
            (Lang::En, Section::Notifications) => "Notifications",
            (Lang::En, Section::Permissions) => "Permissions",
            (_, Section::Diagnostics) => catalog.diagnostics().title(),
            (Lang::En, Section::About) => "About",
            (Lang::En, Section::Overview) => "Overview",
            (Lang::En, Section::Projects) => "Projects",
            (Lang::En, Section::Connections) => "Connections",
            (Lang::En, Section::Devices) => "Pair device",
            (Lang::En, Section::Agents) => "Agents",
            (Lang::En, Section::Metadata) => "Metadata",
            (Lang::En, Section::Voice) => "Voice",
            (Lang::En, Section::Workspaces) => "Workspaces",
            (Lang::En, Section::Providers) => "Providers",
            (Lang::En, Section::Usage) => "Usage",
            (Lang::En, Section::Terminals) => "Terminals",
            (Lang::En, Section::Plugins) => "Plugins",
            (Lang::En, Section::Models) => "Models",
            (Lang::En, Section::Sync) => "Sync",
            (Lang::Ko, Section::General) => "일반",
            (Lang::Ko, Section::Appearance) => "모양",
            (Lang::Ko, Section::Sidebar) => "사이드바",
            (Lang::Ko, Section::Browser) => "브라우저",
            (Lang::Ko, Section::Editor) => "편집기",
            (Lang::Ko, Section::Files) => "파일",
            (Lang::Ko, Section::Shortcuts) => "단축키",
            (Lang::Ko, Section::Integrations) => "통합",
            (Lang::Ko, Section::Notifications) => "알림",
            (Lang::Ko, Section::Permissions) => "권한",
            (Lang::Ko, Section::About) => "정보",
            (Lang::Ko, Section::Overview) => "개요",
            (Lang::Ko, Section::Projects) => "프로젝트",
            (Lang::Ko, Section::Connections) => "연결",
            (Lang::Ko, Section::Devices) => "기기 페어링",
            (Lang::Ko, Section::Agents) => "에이전트",
            (Lang::Ko, Section::Metadata) => "메타데이터",
            (Lang::Ko, Section::Voice) => "음성",
            (Lang::Ko, Section::Workspaces) => "워크스페이스",
            (Lang::Ko, Section::Providers) => "프로바이더",
            (Lang::Ko, Section::Usage) => "사용량",
            // Paseo's Korean names both "터미널" (under different headings).
            (Lang::Ko, Section::Terminal | Section::Terminals) => "터미널",
            (Lang::Ko, Section::Plugins) => "플러그인",
            (Lang::Ko, Section::Models) => "모델",
            (Lang::Ko, Section::Sync) => "동기화",
        }
    }

    /// A group's heading (Paseo's `settings.groups`). The machine group is
    /// Paseo's "Host" named with Vessel's word for a host, as the rest of
    /// the app names it ("Add a machine", "Switch machine"). Short.
    #[must_use]
    pub const fn group(&self, group: Group) -> &'static str {
        match (self.lang, group) {
            (Lang::En, Group::App) => "App",
            (Lang::En, Group::Machine) => "Machine",
            (Lang::Ko, Group::App) => "앱",
            (Lang::Ko, Group::Machine) => "머신",
        }
    }

    /// A theme's name.
    #[must_use]
    pub const fn theme(&self, theme: Theme) -> &'static str {
        match theme {
            Theme::System => self.system,
            Theme::Light => self.light,
            Theme::Dark => self.dark,
            Theme::Zinc => self.zinc,
            Theme::Midnight => self.midnight,
            Theme::Claude => self.claude,
            Theme::Ghostty => self.ghostty,
            Theme::PureBlack => self.pure_black,
        }
    }

    /// A delivery's name.
    #[must_use]
    pub const fn delivery(&self, delivery: TurnDelivery) -> &'static str {
        match delivery {
            TurnDelivery::Queue => self.queue,
            TurnDelivery::Steer => self.steer,
            TurnDelivery::Interrupt => self.interrupt,
        }
    }

    /// What a delivery does (General's note under Default send).
    #[must_use]
    pub const fn delivery_note(&self, delivery: TurnDelivery) -> &'static str {
        match (self.lang, delivery) {
            (Lang::En, TurnDelivery::Queue) => {
                "A message sent while the agent works waits, and goes when its turn ends; while a permission request waits, it stops the turn instead."
            }
            (Lang::En, TurnDelivery::Steer) => {
                "A message sent while the agent works joins its turn, where the agent can take one; otherwise it stops the turn and starts the next."
            }
            (Lang::En, TurnDelivery::Interrupt) => {
                "A message sent while the agent works stops its turn and starts the next."
            }
            (Lang::Ko, TurnDelivery::Queue) => {
                "에이전트가 일하는 동안 보낸 메시지는 기다렸다가, 차례가 끝나면 갑니다. 권한 요청이 기다리는 동안에는 지금 차례를 멈추고 갑니다."
            }
            (Lang::Ko, TurnDelivery::Steer) => {
                "에이전트가 일하는 동안 보낸 메시지는, 에이전트가 받을 수 있으면 지금 차례에 합류하고 아니면 지금 차례를 멈추고 다음 차례를 시작합니다."
            }
            (Lang::Ko, TurnDelivery::Interrupt) => {
                "에이전트가 일하는 동안 보낸 메시지는 지금 차례를 멈추고 다음 차례를 시작합니다."
            }
        }
    }

    /// A width's name.
    #[must_use]
    pub const fn width(&self, width: ContentWidth) -> &'static str {
        match width {
            ContentWidth::Comfortable => self.comfortable,
            ContentWidth::Wide => self.wide,
            ContentWidth::Full => self.full,
        }
    }

    /// A text size as its picker shows it.
    #[must_use]
    pub fn points(&self, points: u8) -> String {
        format!("{points} px")
    }

    /// Why the preferences are kept for this launch only (the page's
    /// `keptNote`); `None` when they are kept in a file.
    #[must_use]
    pub fn kept_why(&self, kept: &Kept) -> Option<String> {
        let Kept::ThisLaunch { why } = kept else {
            return None;
        };
        let reason = match (self.lang, why) {
            (Lang::En, LaunchOnly::NoPlace) => {
                "this computer gives Vessel no place to keep them.".to_owned()
            }
            (Lang::Ko, LaunchOnly::NoPlace) => "이 컴퓨터에 보관할 곳이 없습니다.".to_owned(),
            (_, LaunchOnly::NoPath { message }) => message.clone(),
            (Lang::En, LaunchOnly::Unreadable { fault }) => {
                format!(
                    "{} could not be read ({}). Fix or remove it, then restart Vessel.",
                    fault.path,
                    self.lang.file_problem(fault)
                )
            }
            // No particle after the path (ADR 0039's interim rule).
            (Lang::Ko, LaunchOnly::Unreadable { fault }) => {
                format!(
                    "다음 파일을 읽지 못했습니다: {} ({}). 고치거나 지운 뒤 Vessel을 다시 시작하세요.",
                    fault.path,
                    self.lang.file_problem(fault)
                )
            }
            (Lang::En, LaunchOnly::Newer { path, version }) => {
                format!(
                    "{path} was written by a newer Vessel (version {version}); it is left as it is."
                )
            }
            (Lang::Ko, LaunchOnly::Newer { path, version }) => {
                format!("{path}은(는) 더 새 Vessel(버전 {version})이 썼습니다. 그대로 둡니다.")
            }
        };
        Some(match self.lang {
            Lang::En => format!("Kept until Vessel quits: {reason}"),
            Lang::Ko => format!("Vessel을 끝낼 때까지만 보관합니다: {reason}"),
        })
    }
}
