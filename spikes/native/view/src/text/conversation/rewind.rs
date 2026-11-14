//! The words of rewinding a conversation (protocol 14), Paseo's own in
//! both languages (`i18n/resources/{en,ko}.ts`, `rewind` and
//! `message.actions`): the person's message's trailing row and its
//! rewind menu.

use super::ConversationWords;

impl ConversationWords {
    /// The rewind button's tooltip (Paseo's `rewind.tooltip`).
    #[must_use]
    pub const fn rewind_tooltip(&self) -> &'static str {
        self.either("Rewind to this message", "이 메시지로 되감기")
    }

    /// The menu's header (Paseo's `rewind.warning`).
    #[must_use]
    pub const fn rewind_warning(&self) -> &'static str {
        self.either(
            "This action cannot be undone",
            "이 작업은 되돌릴 수 없습니다",
        )
    }

    /// The menu's row (Paseo's `rewind.actions.conversation`): our node
    /// rewinds the conversation only; files are not put back.
    #[must_use]
    pub const fn rewind_conversation(&self) -> &'static str {
        self.either("Rewind conversation", "대화 되감기")
    }

    /// A rewind the node refused, with its words (Paseo's toast: the
    /// error's message, else `rewind.errors.failed`).
    #[must_use]
    pub fn rewind_failed(&self, message: &str) -> String {
        let failed = self.either("Failed to rewind agent", "에이전트를 되감지 못했습니다");
        if message.trim().is_empty() {
            failed.to_owned()
        } else {
            format!("{failed}: {message}")
        }
    }

    /// The person's message's copy button (Paseo's
    /// `message.actions.copyMessage`).
    #[must_use]
    pub const fn copy_message(&self) -> &'static str {
        self.either("Copy message", "메시지 복사")
    }
}
