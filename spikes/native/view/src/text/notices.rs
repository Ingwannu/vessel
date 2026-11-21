//! The notices' words (the page's `data/notices.ts`): the titles of the
//! system's notifications; in English and Korean. See `text/mod.rs` for
//! the convention.

use super::Lang;
use crate::notices::NoticeKind;

impl Lang {
    /// The notices' words in this language.
    #[must_use]
    pub const fn notices(self) -> &'static NoticeWords {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

/// One language's sentences.
#[derive(Debug)]
#[expect(missing_docs, reason = "each field is named for the sentence it holds")]
pub struct NoticeWords {
    pub lang: Lang,
    needs_you: &'static str,
    finished: &'static str,
    terminal_needs_you: &'static str,
    terminal_finished: &'static str,
}

impl NoticeWords {
    /// A notice's title, in GOAT's words with Vessel's "session".
    #[must_use]
    pub const fn title(&self, kind: NoticeKind) -> &'static str {
        match kind {
            NoticeKind::NeedsYou => self.needs_you,
            NoticeKind::Finished => self.finished,
        }
    }

    /// A terminal notice's title, in Paseo's words
    /// (`websocket-server.ts:213-215`).
    #[must_use]
    pub const fn terminal_title(&self, kind: NoticeKind) -> &'static str {
        match kind {
            NoticeKind::NeedsYou => self.terminal_needs_you,
            NoticeKind::Finished => self.terminal_finished,
        }
    }
}

const EN: NoticeWords = NoticeWords {
    lang: Lang::En,
    needs_you: "Session needs you",
    finished: "Session finished",
    terminal_needs_you: "Terminal needs input",
    terminal_finished: "Terminal finished",
};

const KO: NoticeWords = NoticeWords {
    lang: Lang::Ko,
    needs_you: "세션이 응답을 기다립니다",
    finished: "세션이 끝났습니다",
    terminal_needs_you: "터미널이 입력을 기다립니다",
    terminal_finished: "터미널이 끝났습니다",
};
