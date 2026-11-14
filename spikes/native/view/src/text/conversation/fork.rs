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
use crate::timeline::EmptyFork;

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

    /// The banner's heading over a fork that never held a conversation.
    #[must_use]
    pub const fn empty_fork_heading(&self) -> &'static str {
        self.either("The fork did not start", "분기가 시작되지 않았습니다")
    }

    /// Why, under the heading: A2's sentence for the point refused, and
    /// that nothing can go on here.
    #[must_use]
    pub fn empty_fork_line(&self, why: &EmptyFork) -> String {
        match why {
            EmptyFork::NotAtPoint => self
                .either(
                    "The agent did not fork at the chosen point. Nothing here can go on; remove it and fork again.",
                    "에이전트가 고른 지점에서 분기하지 않았습니다. 여기서는 이어 갈 수 없으니 삭제하고 다시 분기하세요.",
                )
                .to_owned(),
            EmptyFork::CannotFork => self
                .either(
                    "This agent cannot fork a conversation. Nothing here can go on.",
                    "이 에이전트는 대화를 분기할 수 없습니다. 여기서는 이어 갈 수 없습니다.",
                )
                .to_owned(),
            EmptyFork::Other(message) => message.clone(),
        }
    }

    /// The banner's button: the empty fork removed (`session.delete`).
    #[must_use]
    pub const fn remove_fork(&self) -> &'static str {
        self.either("Remove", "삭제")
    }

    /// Under a fork's first line when its source's conversation cannot be
    /// shown: `removed` when the node no longer has the session, else
    /// the node's or the connection's `message`.
    #[must_use]
    pub fn origin_unread(&self, removed: bool, message: &str) -> String {
        if removed {
            return self
                .either(
                    "The conversation it was forked from was removed.",
                    "분기한 원래 대화는 삭제되었습니다.",
                )
                .to_owned();
        }
        let failed = self.either(
            "The conversation it was forked from could not be read",
            "분기한 원래 대화를 읽지 못했습니다",
        );
        let again = self.either(
            "Open the session again to try again.",
            "세션을 다시 열면 다시 읽습니다.",
        );
        let message = message.trim().trim_end_matches('.');
        if message.is_empty() {
            format!("{failed}. {again}")
        } else {
            format!("{failed}: {message}. {again}")
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
