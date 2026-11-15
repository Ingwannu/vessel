//! The words of the permission card over the composer: Paseo's
//! (`agentStream.permission.*`, its English and Korean), and GOAT's count.

use super::ConversationWords;
use crate::Lang;

impl ConversationWords {
    /// The timeline's line for a permission while its card waits over the
    /// composer.
    #[must_use]
    pub const fn permission_waiting(&self) -> &'static str {
        self.either("Waiting on permission", "권한 대기 중")
    }

    /// The permission card's title when the agent gave none of its own.
    #[must_use]
    pub const fn permission_required(&self) -> &'static str {
        self.either("Permission Required", "권한 필요")
    }

    /// The permission card's title for a plan to approve.
    #[must_use]
    pub const fn permission_plan(&self) -> &'static str {
        self.either("Plan", "계획")
    }

    /// The permission card's line over its answers.
    #[must_use]
    pub const fn permission_question(&self) -> &'static str {
        self.either("How would you like to proceed?", "어떻게 진행할까요?")
    }

    /// At the permission card's top right when several wait: which one is
    /// shown, of how many (GOAT's "1/3").
    #[must_use]
    pub fn permission_count(&self, at: usize, total: usize) -> String {
        match self.lang {
            Lang::En | Lang::Ko => format!("{at}/{total}"),
        }
    }
}
