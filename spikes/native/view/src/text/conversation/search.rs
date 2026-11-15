//! The words of the palette's Messages group (GOAT's message rows,
//! `ThreadSearchMatch.tsx`, "You:" and "Agent:"; the group's notes are the
//! app's own: GOAT has no machine too old to search).

use crate::Lang;

use super::ConversationWords;

impl ConversationWords {
    /// The palette group's heading.
    #[must_use]
    pub const fn search_group(&self) -> &'static str {
        self.either("Messages", "메시지")
    }

    /// Before a person's message's snippet in a row.
    #[must_use]
    pub const fn search_you(&self) -> &'static str {
        self.either("You:", "나:")
    }

    /// Before the agent's message's snippet in a row.
    #[must_use]
    pub const fn search_agent(&self) -> &'static str {
        self.either("Agent:", "에이전트:")
    }

    /// The row after the messages shown, when more may match.
    #[must_use]
    pub const fn search_more(&self) -> &'static str {
        self.either("Show more messages", "메시지 더 보기")
    }

    /// Under the group while no machine has answered.
    #[must_use]
    pub const fn search_searching(&self) -> &'static str {
        self.either("Searching messages…", "메시지를 찾는 중…")
    }

    /// Under the group when every machine answered and none matched.
    #[must_use]
    pub const fn search_nothing(&self) -> &'static str {
        self.either("No messages match.", "일치하는 메시지가 없습니다.")
    }

    /// Under the group when the words are not a search a machine takes.
    #[must_use]
    pub const fn search_invalid(&self) -> &'static str {
        self.either(
            "Search with up to 256 characters and 16 words.",
            "검색어는 256자, 16단어까지 쓸 수 있습니다.",
        )
    }

    /// Under the group: the machines too old to search, by name.
    #[must_use]
    pub fn search_older(&self, machines: &[String]) -> String {
        let names = machines.join(", ");
        match self.lang {
            Lang::En => format!("Not searched (needs a newer Vessel): {names}"),
            Lang::Ko => format!("검색하지 않은 머신(새 Vessel 필요): {names}"),
        }
    }

    /// Under the group: a machine that could not search, and its words.
    #[must_use]
    pub fn search_failed(&self, machine: &str, why: &str) -> String {
        match self.lang {
            Lang::En => format!("Could not search {machine}: {why}"),
            Lang::Ko => format!("검색 실패 · {machine}: {why}"),
        }
    }
}
