//! The words of forking a conversation (protocol 14), Paseo's own in both
//! languages where it has them (`i18n/resources/{en,ko}.ts`,
//! `message.actions`): the turn footer's fork menu, and a forked
//! session's first line.
//!
//! Paseo's menu forks "in a new tab" or "in a new workspace"; our node
//! forks into a new session beside the source, so the one row says that,
//! in the same form.

use super::ConversationWords;
use crate::Lang;

impl ConversationWords {
    /// The fork button's tooltip (Paseo's `message.actions.forkMenu`).
    #[must_use]
    pub const fn fork_tooltip(&self) -> &'static str {
        self.either("Fork chat from here", "여기에서 채팅 분기")
    }

    /// The menu's row (Paseo's `forkInNewTab`, for our one target).
    #[must_use]
    pub const fn fork_in_new_session(&self) -> &'static str {
        self.either("Fork in a new session", "새 세션으로 분기")
    }

    /// A fork the node refused, with its words (Paseo's toast: the error's
    /// message, else `message.actions.forkFailed`).
    #[must_use]
    pub fn fork_failed(&self, message: &str) -> String {
        let failed = self.either("Failed to fork chat", "채팅을 분기하지 못했습니다");
        if message.trim().is_empty() {
            failed.to_owned()
        } else {
            format!("{failed}: {message}")
        }
    }

    /// A forked session's first line: the source's title when the list
    /// knows it, else that it came from another session.
    #[must_use]
    pub fn forked_from(&self, source: Option<&str>) -> String {
        match source.map(str::trim).filter(|title| !title.is_empty()) {
            Some(title) => match self.lang {
                Lang::En => format!("Forked from “{title}”"),
                Lang::Ko => format!("“{title}”에서 분기함"),
            },
            None => self
                .either("Forked from another session", "다른 세션에서 분기함")
                .to_owned(),
        }
    }
}
