//! Importing from an agent's own history (protocol 14, milestone 11's
//! track A2, ADR 0053): `agent.history`, a page of the conversations an
//! agent keeps, as it lists them over ACP.
//!
//! [Decision Log]
//! - Purpose: a person brings a conversation they had with an agent
//!   outside Vessel (in its own terminal, another client) into a session,
//!   as Paseo, agent-orchestrator and GOAT offer.
//! - Alternatives: reading each harness's files (agent-orchestrator's and
//!   GOAT's readers, one per harness, broken by any change of format);
//!   Paseo's listing with previews (every listed conversation loaded to
//!   show its first and last prompts, up to 60 s a listing).
//! - Choice: the agent lists its own (ACP `session/list`), a page of its
//!   per call (up to [`AgentHistoryPage::MAX_ENTRIES`], a larger one in
//!   parts) behind a cursor the node mints; no previews (the agent's title
//!   and time); the import is `session.create` with `resume`, whose load
//!   replays the conversation into the new session's log.
//! - Scope `operate`, as the import it serves: titles are usually a
//!   conversation's first prompt, from every folder the account worked
//!   in, which a read-only device does not otherwise see.
//! - Cost: only an agent that lists over ACP (and loads) offers anything;
//!   each call starts the agent once (seconds).

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::agents::HarnessRef;
use crate::id::{HarnessSessionId, SessionId};
use crate::name::ProfileName;
use crate::title::SessionTitle;

/// `agent.history`: a page of `harness`'s own conversations, as its agent
/// lists them. The node starts the agent as `harness.options` does (under
/// `profile`, in a scratch folder), asks it, and stops it; no session is
/// started or loaded. Needs `operate`; refused to agents; at most two at
/// once on a node (`unavailable` beyond).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetAgentHistory {
    /// Which agent; an added agent needs protocol 11.
    pub harness: HarnessRef,
    /// Under this profile (another account keeps other conversations);
    /// `None` is the harness's default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<ProfileName>,
    /// Only the conversations held in this folder (absolute, on the
    /// node; the node resolves its links); all of them without.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,
    /// Where the page starts: a page's `next`, as it was given (at most
    /// [`GetAgentHistory::MAX_CURSOR_BYTES`]; anything else is
    /// `invalid_request`); the newest without.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

impl GetAgentHistory {
    /// The longest cursor the node takes.
    pub const MAX_CURSOR_BYTES: usize = 4096;
}

/// Reply to `agent.history`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AgentHistoryPage {
    /// Newest first, as the agent gives them; at most
    /// [`AgentHistoryPage::MAX_ENTRIES`].
    pub conversations: Vec<HistoryEntry>,
    /// The next page's cursor, opaque; absent on the last page. A page
    /// can be empty and still have one (an agent that filters by folder
    /// after paging), after the node has asked several of the agent's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
}

impl AgentHistoryPage {
    /// The most conversations one page carries (Claude's adapter lists
    /// 1,000 a page; a larger page of an agent's comes in parts).
    pub const MAX_ENTRIES: usize = 1000;
}

/// One conversation an agent keeps. Importing it is `session.create` with
/// `resume: harness_session`, its `cwd`, the `profile` it was listed under
/// (the conversation is in that account's files), and its `title` (an
/// imported session is otherwise untitled until its next prompt). The
/// import is refused `conflict` while another live session is on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct HistoryEntry {
    /// The agent's id for it.
    pub harness_session: HarnessSessionId,
    /// The folder it ran in, absolute.
    pub cwd: PathBuf,
    /// Its title as the agent gives it, cut as a prompt's title is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<SessionTitle>,
    /// When it was last active, as the agent says it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    /// The newest session on this node, live or kept, on this
    /// conversation of this agent: a client opens it (`session.resume` for
    /// an ended one) rather than import it twice. A rewound session is on
    /// its new conversation, so the one it left lists unmarked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_vessel: Option<SessionId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_reads_as_it_is_written_and_leaves_out_what_it_lacks() {
        let page = AgentHistoryPage {
            conversations: vec![HistoryEntry {
                harness_session: HarnessSessionId::new("past-2"),
                cwd: PathBuf::from("/work/app"),
                title: None,
                updated_at: None,
                in_vessel: None,
            }],
            next: None,
        };
        let text = serde_json::to_string(&page).unwrap();
        assert_eq!(
            text,
            r#"{"conversations":[{"harness_session":"past-2","cwd":"/work/app"}]}"#
        );
        assert_eq!(
            serde_json::from_str::<AgentHistoryPage>(&text).unwrap(),
            page
        );
        let asked: GetAgentHistory = serde_json::from_str(r#"{"harness":"claude-code"}"#).unwrap();
        assert_eq!(asked.profile, None);
        assert_eq!(asked.cursor, None);
    }
}
