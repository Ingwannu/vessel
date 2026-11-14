//! The words of Settings' frame and of its General, Appearance and About
//! sections, in English and Korean (see `text/mod.rs` for the
//! convention). The page's words (`features/settings/*.tsx`), with "this
//! page" made "Vessel" where the app is meant.

use super::Lang;
use crate::prefs::{ContentWidth, ConversationDisplay, Kept, LaunchOnly, Theme, TurnDelivery};
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
    pub elsewhere: &'static str,
    // General.
    pub general_lead: &'static str,
    pub composer: &'static str,
    pub send_shortcut: &'static str,
    pub enter_sends: &'static str,
    pub shortcut_sends: &'static str,
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
    pub typography: &'static str,
    pub interface_font: &'static str,
    pub interface_font_note: &'static str,
    pub code_font: &'static str,
    pub code_font_note: &'static str,
    pub conversation: &'static str,
    pub content_width: &'static str,
    pub content_width_note: &'static str,
    pub comfortable: &'static str,
    pub wide: &'static str,
    pub full: &'static str,
    pub conversation_display: &'static str,
    pub conversation_display_note: &'static str,
    pub collapse_tools: &'static str,
    // About.
    pub about_lead: &'static str,
    pub version: &'static str,
    pub licence: &'static str,
    pub licences: &'static str,
    pub machines_versions: &'static str,
}

const EN: SettingsText = SettingsText {
    lang: Lang::En,
    settings: "Settings",
    back: "Back",
    machine: "Machine",
    no_machines: "No machines yet.",
    switch_machine: "Switch machine",
    elsewhere: "This section is drawn by another part of the app; it comes in a later build.",
    general_lead: "How the composer sends, and where Vessel keeps what you choose.",
    composer: "Composer",
    send_shortcut: "Send shortcut",
    enter_sends: "Enter sends a prompt; Shift+Enter starts a new line.",
    shortcut_sends: "Enter starts a new line; the shortcut sends.",
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
    typography: "Typography",
    interface_font: "Interface font",
    interface_font_note: "Everything outside code blocks and the terminal.",
    code_font: "Code font",
    code_font_note: "Code blocks, tool output and the terminal.",
    conversation: "Conversation",
    content_width: "Content width",
    content_width_note: "How wide the conversation and the composer grow on large screens.",
    comfortable: "Comfortable",
    wide: "Wide",
    full: "Full",
    conversation_display: "Conversation display",
    conversation_display_note: "How much of an agent's work the conversation shows.",
    collapse_tools: "Collapse tool calls",
    about_lead: "This app's version and licence, and each machine's.",
    version: "Version",
    licence: "Licence",
    licences: "Licences",
    machines_versions: "Machines",
};

const KO: SettingsText = SettingsText {
    lang: Lang::Ko,
    settings: "설정",
    back: "뒤로",
    machine: "머신",
    no_machines: "아직 머신이 없습니다.",
    switch_machine: "머신 바꾸기",
    elsewhere: "이 섹션은 앱의 다른 부분이 그립니다. 다음 빌드에서 들어옵니다.",
    general_lead: "작성창이 보내는 방식과, 고른 설정을 Vessel이 보관하는 곳.",
    composer: "작성창",
    send_shortcut: "보내기 단축키",
    enter_sends: "Enter로 보내고, Shift+Enter로 줄을 바꿉니다.",
    shortcut_sends: "Enter로 줄을 바꾸고, 단축키로 보냅니다.",
    default_send: "기본 보내기",
    queue: "대기열",
    steer: "조종",
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
    typography: "글꼴",
    interface_font: "인터페이스 글꼴",
    interface_font_note: "코드 블록과 터미널을 뺀 모든 곳.",
    code_font: "코드 글꼴",
    code_font_note: "코드 블록, 도구 출력, 터미널.",
    conversation: "대화",
    content_width: "내용 너비",
    content_width_note: "큰 화면에서 대화와 작성창이 넓어지는 정도.",
    comfortable: "편안하게",
    wide: "넓게",
    full: "가득",
    conversation_display: "대화 표시",
    conversation_display_note: "에이전트의 작업을 대화에 얼마나 보여 줄지.",
    collapse_tools: "도구 호출 접기",
    about_lead: "이 앱과 각 머신의 버전과 라이선스.",
    version: "버전",
    licence: "라이선스",
    licences: "라이선스",
    machines_versions: "머신",
};

impl SettingsText {
    /// A section's name in the list.
    #[must_use]
    pub const fn section(&self, section: Section) -> &'static str {
        match (self.lang, section) {
            (Lang::En, Section::General) => "General",
            (Lang::En, Section::Appearance) => "Appearance",
            (Lang::En, Section::Shortcuts) => "Shortcuts",
            (Lang::En, Section::Notifications) => "Notifications",
            (Lang::En, Section::Agents) => "Agents",
            (Lang::En, Section::Models) => "Models",
            (Lang::En, Section::Machines) => "Machines",
            (Lang::En, Section::Projects) => "Projects",
            (Lang::En, Section::Devices) => "Devices",
            (Lang::En, Section::Sync) => "Sync",
            (Lang::En, Section::About) => "About",
            (Lang::Ko, Section::General) => "일반",
            (Lang::Ko, Section::Appearance) => "모양",
            (Lang::Ko, Section::Shortcuts) => "단축키",
            (Lang::Ko, Section::Notifications) => "알림",
            (Lang::Ko, Section::Agents) => "에이전트",
            (Lang::Ko, Section::Models) => "모델",
            (Lang::Ko, Section::Machines) => "머신",
            (Lang::Ko, Section::Projects) => "프로젝트",
            (Lang::Ko, Section::Devices) => "기기",
            (Lang::Ko, Section::Sync) => "동기화",
            (Lang::Ko, Section::About) => "정보",
        }
    }

    /// A group's heading.
    #[must_use]
    pub const fn group(&self, group: Group) -> &'static str {
        match (self.lang, group) {
            (Lang::En, Group::App) => "App",
            (Lang::En, Group::Agents) => "Agents",
            (Lang::En, Group::Machines) => "Machines",
            (Lang::En, Group::About) => "About",
            (Lang::Ko, Group::App) => "앱",
            (Lang::Ko, Group::Agents) => "에이전트",
            (Lang::Ko, Group::Machines) => "머신",
            (Lang::Ko, Group::About) => "정보",
        }
    }

    /// A theme's name.
    #[must_use]
    pub const fn theme(&self, theme: Theme) -> &'static str {
        match theme {
            Theme::System => self.system,
            Theme::Light => self.light,
            Theme::Dark => self.dark,
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

    /// A display's name.
    #[must_use]
    pub const fn display(&self, display: ConversationDisplay) -> &'static str {
        match display {
            ConversationDisplay::Full => self.full,
            ConversationDisplay::CollapseTools => self.collapse_tools,
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
