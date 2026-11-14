//! The shortcuts' words (the page's `data/shortcuts/registry.ts` labels
//! and `edit.ts` sentences): every action's name, Settings' groups, why a
//! key is refused and what it clashes with; in English and Korean. See
//! `text/mod.rs` for the convention.

use super::Lang;
use crate::shortcuts::{Action, Clash, Platform, Refusal, ShortcutGroup};

impl Lang {
    /// The shortcuts' words in this language.
    #[must_use]
    pub const fn shortcuts(self) -> &'static ShortcutWords {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

/// One language's sentences.
#[derive(Debug)]
#[expect(missing_docs, reason = "each field is named for the sentence it holds")]
pub struct ShortcutWords {
    pub lang: Lang,
    /// Between a sequence's two chords ("G then S").
    pub then: &'static str,
    labels: [&'static str; 16],
    groups: [&'static str; 4],
    mac_modifier: &'static str,
    other_modifier: &'static str,
    kept: &'static str,
    editing: &'static str,
}

impl ShortcutWords {
    /// An action's name, as Settings and the palette show it.
    #[must_use]
    pub fn label(&self, action: Action) -> &'static str {
        let at = Action::ALL
            .iter()
            .position(|one| *one == action)
            .unwrap_or_default();
        self.labels.get(at).copied().unwrap_or_default()
    }

    /// A Settings group's heading.
    #[must_use]
    pub fn group(&self, group: ShortcutGroup) -> &'static str {
        let at = ShortcutGroup::ALL
            .iter()
            .position(|one| *one == group)
            .unwrap_or_default();
        self.groups.get(at).copied().unwrap_or_default()
    }

    /// Why a recorded key is refused.
    #[must_use]
    pub const fn refusal(&self, refusal: Refusal) -> &'static str {
        match refusal {
            Refusal::NoModifier(Platform::Mac) => self.mac_modifier,
            Refusal::NoModifier(Platform::Other) => self.other_modifier,
            Refusal::Kept => self.kept,
            Refusal::Editing => self.editing,
        }
    }

    /// The sentence a clash shows: `binding` in words, `to` the action
    /// the key was recorded for.
    #[must_use]
    pub fn clash(&self, clash: Clash, binding: &str, to: Action) -> String {
        match (self.lang, clash) {
            (Lang::En, Clash::Same(_)) => format!("{binding} is already a key of this action."),
            (Lang::En, Clash::Fixed(held)) => {
                format!("{binding} is used by {}.", self.label(held))
            }
            (Lang::En, Clash::Taken(held)) => format!(
                "{binding} is assigned to {}. Reassign it to {}?",
                self.label(held),
                self.label(to)
            ),
            (Lang::Ko, Clash::Same(_)) => format!("{binding} 키는 이미 이 동작의 키입니다."),
            (Lang::Ko, Clash::Fixed(held)) => {
                format!("{binding} 키는 ‘{}’에 쓰입니다.", self.label(held))
            }
            (Lang::Ko, Clash::Taken(held)) => format!(
                "{binding} 키는 ‘{}’에 지정되어 있습니다. ‘{}’에 다시 지정할까요?",
                self.label(held),
                self.label(to)
            ),
        }
    }
}

const EN: ShortcutWords = ShortcutWords {
    lang: Lang::En,
    then: " then ",
    labels: [
        "Open the command palette",
        "New session",
        "Open Settings",
        "Go to the board",
        "Go to Machines",
        "Go to Sync",
        "Go to Devices",
        "Previous session",
        "Next session",
        "Go to session 1–9",
        "Find in the session",
        "Focus the composer",
        "Send the first queued message now",
        "Edit the last queued message",
        "Show or hide the sidebar",
        "Show or hide the summary",
    ],
    groups: ["General", "Navigation", "Session", "Panels"],
    mac_modifier: "Use Control, Option or Command with another key.",
    other_modifier: "Use Ctrl or Alt with another key.",
    kept: "The system keeps that shortcut for itself.",
    editing: "That shortcut is reserved for standard editing commands.",
};

const KO: ShortcutWords = ShortcutWords {
    lang: Lang::Ko,
    then: " 다음 ",
    labels: [
        "명령 팔레트 열기",
        "새 세션",
        "설정 열기",
        "보드로 이동",
        "머신으로 이동",
        "동기화로 이동",
        "기기로 이동",
        "이전 세션",
        "다음 세션",
        "세션 1–9로 이동",
        "세션에서 찾기",
        "입력창으로 이동",
        "대기 중인 첫 메시지 지금 보내기",
        "마지막 대기 메시지 편집",
        "사이드바 보이기·숨기기",
        "요약 보이기·숨기기",
    ],
    groups: ["일반", "이동", "세션", "패널"],
    mac_modifier: "Control, Option 또는 Command를 다른 키와 함께 누르세요.",
    other_modifier: "Ctrl 또는 Alt를 다른 키와 함께 누르세요.",
    kept: "시스템이 쓰는 단축키입니다.",
    editing: "기본 편집 명령에 쓰이는 단축키입니다.",
};
