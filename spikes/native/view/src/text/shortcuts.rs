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
    labels: [&'static str; Action::ALL.len()],
    groups: [&'static str; 4],
    mac_modifier: &'static str,
    other_modifier: &'static str,
    kept: &'static str,
    editing: &'static str,
    /// Settings › Shortcuts, a row recording: in its keys' place until a
    /// key is pressed (Paseo's `capturePrompt`). Short.
    pub capture_prompt: &'static str,
    /// A row whose action has no key (Paseo's `unassigned`). Short.
    pub unassigned: &'static str,
    /// The row menu's first item when the action has no key. One word.
    pub bind: &'static str,
    /// The row menu's first item when the action has a key. One word.
    pub rebind: &'static str,
    /// The row menu's item that leaves the action with no key. One word.
    pub clear: &'static str,
    /// The row menu's item that gives the action its default key back.
    pub reset: &'static str,
    /// At the first section's head while any key was changed. Short.
    pub reset_all: &'static str,
    /// A recording row's button that saves the key. One word.
    pub done: &'static str,
    /// A recording row's button, and the dialog's, that drops the key.
    pub cancel: &'static str,
    /// The title of the question before a key moves from another action.
    pub in_use: &'static str,
    /// That question's button that moves it. One word.
    pub reassign: &'static str,
    /// Between an action's two keys on its row ("Ctrl+K or /"). One word.
    pub or: &'static str,
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
        "Cycle theme",
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
    capture_prompt: "Press shortcut…",
    unassigned: "Not set",
    bind: "Bind",
    rebind: "Rebind",
    clear: "Clear",
    reset: "Reset to default",
    reset_all: "Reset all",
    done: "Done",
    cancel: "Cancel",
    in_use: "Shortcut already in use",
    reassign: "Reassign",
    or: "or",
};

const KO: ShortcutWords = ShortcutWords {
    lang: Lang::Ko,
    then: " 다음 ",
    labels: [
        "명령 팔레트 열기",
        "새 세션",
        "설정 열기",
        "테마 전환",
        "보드로 이동",
        "머신으로 이동",
        "동기화로 이동",
        "기기로 이동",
        "이전 세션",
        "다음 세션",
        "세션 1–9로 이동",
        "세션에서 찾기",
        "입력창으로 이동",
        "첫 대기 메시지 보내기",
        "마지막 대기 메시지 편집",
        "사이드바 보이기·숨기기",
        "요약 보이기·숨기기",
    ],
    groups: ["일반", "이동", "세션", "패널"],
    mac_modifier: "Control, Option 또는 Command를 다른 키와 함께 누르세요.",
    other_modifier: "Ctrl 또는 Alt를 다른 키와 함께 누르세요.",
    kept: "시스템이 쓰는 단축키입니다.",
    editing: "기본 편집 명령에 쓰이는 단축키입니다.",
    capture_prompt: "단축키를 누르세요…",
    unassigned: "설정되지 않음",
    bind: "지정",
    rebind: "다시 지정",
    clear: "지우기",
    reset: "기본값으로 재설정",
    reset_all: "모두 재설정",
    done: "완료",
    cancel: "취소",
    in_use: "이미 쓰이는 단축키",
    reassign: "다시 지정",
    or: "또는",
};
