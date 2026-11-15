//! The Animations setting's words (the owner's word 39), in English and
//! Korean. See `text/mod.rs` for the convention.

use super::Lang;

impl Lang {
    /// The motion setting's words in this language.
    #[must_use]
    pub const fn motion(self) -> &'static MotionWords {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

/// One language's sentences.
#[derive(Debug)]
pub struct MotionWords {
    /// Settings › Appearance: the section's small title. Short.
    pub section: &'static str,
    /// The row's title, beside its switch. One word.
    pub animations: &'static str,
    /// The row's hint under its title: what the switch changes, one
    /// sentence (wraps on a phone).
    pub note: &'static str,
}

const EN: MotionWords = MotionWords {
    section: "Motion",
    animations: "Animations",
    note: "Short transitions when something opens, closes or moves. Off shows every change at once.",
};

const KO: MotionWords = MotionWords {
    section: "움직임",
    animations: "애니메이션",
    note: "무엇이 열리고 닫히거나 움직일 때 짧게 전환합니다. 끄면 모든 변화가 바로 보입니다.",
};
