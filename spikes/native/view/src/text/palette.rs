//! The command palette's words (the page's `data/commands.ts` and
//! `ui/kit/command.tsx`): its groups, its actions and places, its search
//! line and foot; in English and Korean. See `text/mod.rs` for the
//! convention.

use super::Lang;
use crate::palette::PaletteGroup;

impl Lang {
    /// The palette's words in this language.
    #[must_use]
    pub const fn palette(self) -> &'static PaletteWords {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

/// One language's sentences.
#[derive(Debug)]
#[expect(missing_docs, reason = "each field is named for the sentence it holds")]
pub struct PaletteWords {
    pub lang: Lang,
    pub title: &'static str,
    pub placeholder: &'static str,
    pub nothing: &'static str,
    pub new_session: &'static str,
    pub add_machine: &'static str,
    pub board: &'static str,
    /// The sidebar's row that opens the palette (Paseo's
    /// `sidebar.sections.search`); one short word, beside a glyph.
    pub search: &'static str,
    // The foot.
    pub move_keys: &'static str,
    pub open: &'static str,
    pub close: &'static str,
    groups: [&'static str; 5],
    themes: [&'static str; 3],
}

impl PaletteWords {
    /// A group's heading.
    #[must_use]
    pub fn group(&self, group: PaletteGroup) -> &'static str {
        let at = PaletteGroup::ALL
            .iter()
            .position(|one| *one == group)
            .unwrap_or_default();
        self.groups.get(at).copied().unwrap_or_default()
    }

    /// "Settings: Shortcuts", for a section's name.
    #[must_use]
    pub fn settings(&self, section: &str) -> String {
        match self.lang {
            Lang::En => format!("Settings: {section}"),
            Lang::Ko => format!("설정: {section}"),
        }
    }

    /// "Theme: dark", for the choice at `at` (system, light, dark).
    #[must_use]
    pub fn theme(&self, at: usize) -> String {
        let name = self.themes.get(at).copied().unwrap_or_default();
        match self.lang {
            Lang::En => format!("Theme: {name}"),
            Lang::Ko => format!("테마: {name}"),
        }
    }
}

const EN: PaletteWords = PaletteWords {
    lang: Lang::En,
    title: "Command palette",
    placeholder: "Search machines, sessions and commands",
    nothing: "Nothing matches.",
    new_session: "New session",
    add_machine: "Add a machine",
    board: "Board",
    search: "Search",
    move_keys: "move",
    open: "open",
    close: "close",
    groups: ["Actions", "Go to", "Machines", "Sessions", "Theme"],
    themes: ["system", "light", "dark"],
};

const KO: PaletteWords = PaletteWords {
    lang: Lang::Ko,
    title: "명령 팔레트",
    placeholder: "머신, 세션, 명령 검색",
    nothing: "일치하는 항목이 없습니다.",
    new_session: "새 세션",
    add_machine: "머신 추가",
    board: "보드",
    search: "검색",
    move_keys: "이동",
    open: "열기",
    close: "닫기",
    groups: ["동작", "이동", "머신", "세션", "테마"],
    themes: ["시스템", "라이트", "다크"],
};
