//! Importing from an agent's own history (protocol 17, milestone 11's
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
//! - Ranked listing ([`RANKED_HISTORY_PROTOCOL`], ADR 0109, partly
//!   superseding 0053): with `limit`, the node answers what Paseo's
//!   `fetch_recent_provider_sessions` does: the newest `limit`
//!   conversations not already on the node, each with its first and last
//!   prompt read by `session/load` within Paseo's budgets, `query`
//!   matched on the node, and how many already imported were left out.
//!   Paseo's sheet pages by growing `limit` (15, 45, 90, 200), not by a
//!   cursor, because a ranking redone each time cannot drift under an
//!   offset. The cursor listing stays as it was for older clients.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::agents::HarnessRef;
use crate::id::{HarnessSessionId, SessionId};
use crate::name::ProfileName;
use crate::node::ProtocolVersion;
use crate::title::SessionTitle;

/// The protocol of `agent.history`'s ranked listing (`limit`, `query`, the
/// previews and `already_imported`; ADR 0109). Assigned 29 at its node-line
/// merge, the next free number in merge order.
pub const RANKED_HISTORY_PROTOCOL: Option<ProtocolVersion> = Some(ProtocolVersion::new(29));

/// `agent.history`: a page of `harness`'s own conversations, as its agent
/// lists them. The node starts the agent as `harness.options` does (under
/// `profile`, in a scratch folder), asks it, and stops it. A cursor
/// listing starts or loads no session; a ranked one (`limit`) loads each
/// conversation it answers to read its prompts, and starts none. Needs
/// `operate`; refused to agents; at most four at once on a node and
/// sixteen waiting for a place (`unavailable` beyond).
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
    /// `invalid_request`); the newest without. Not with `limit`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// A ranked listing ([`RANKED_HISTORY_PROTOCOL`]): the newest `limit`
    /// conversations (1 to [`GetAgentHistory::MAX_LIMIT`]) not already on
    /// this node, with their prompts, in one answer without `next`. A
    /// client asks again with a larger limit for more (Paseo's 15, 45, 90,
    /// 200).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u16>,
    /// Only the conversations whose title, first or last prompt, or
    /// folder's last name holds this text, letter case aside (trimmed;
    /// at most [`GetAgentHistory::MAX_QUERY_BYTES`]). Needs `limit`; the
    /// node looks through at most [`GetAgentHistory::SEARCH_SCAN`] of
    /// the agent's conversations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

impl GetAgentHistory {
    /// The longest cursor the node takes.
    pub const MAX_CURSOR_BYTES: usize = 4096;
    /// The largest `limit` (Paseo's protocol ceiling).
    pub const MAX_LIMIT: u16 = 200;
    /// The longest `query` the node takes.
    pub const MAX_QUERY_BYTES: usize = 512;
    /// The most of the agent's conversations a ranked listing looks
    /// through (Paseo's scan limit, with or without a query).
    pub const SEARCH_SCAN: usize = 500;

    /// Whether this asks a ranked listing (`limit` or `query` given).
    #[must_use]
    pub const fn is_ranked(&self) -> bool {
        self.limit.is_some() || self.query.is_some()
    }
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
    /// Never on a ranked listing's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    /// A ranked listing's: how many of the conversations it looked
    /// through (and that match its `query`) are already on this node, so
    /// were left out. Paseo's sheet says "All recent sessions are already
    /// imported." when nothing is left and this is not zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub already_imported: Option<u32>,
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
    /// A ranked listing's: the conversation's first prompt, its blanks
    /// folded to one space and cut at
    /// [`HistoryEntry::PROMPT_PREVIEW_CHARS`]; absent when it was not read
    /// in time or the agent replayed none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_prompt: Option<String>,
    /// A ranked listing's: its last prompt, as `first_prompt`. Paseo's
    /// row shows this one, else the first, else "No prompt preview".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_prompt: Option<String>,
}

impl HistoryEntry {
    /// The longest prompt preview, in characters (Paseo's 160).
    pub const PROMPT_PREVIEW_CHARS: usize = 160;
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
                first_prompt: None,
                last_prompt: None,
            }],
            next: None,
            already_imported: None,
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
        assert!(
            !asked.is_ranked(),
            "a cursor listing, as before protocol 29"
        );
    }

    #[test]
    fn a_ranked_listing_carries_its_limit_query_prompts_and_count() {
        let asked: GetAgentHistory =
            serde_json::from_str(r#"{"harness":"claude-code","limit":15,"query":"Fix the build"}"#)
                .unwrap();
        assert!(asked.is_ranked());
        assert_eq!(asked.limit, Some(15));
        let page = AgentHistoryPage {
            conversations: vec![HistoryEntry {
                harness_session: HarnessSessionId::new("past-2"),
                cwd: PathBuf::from("/work/app"),
                title: None,
                updated_at: None,
                in_vessel: None,
                first_prompt: Some("fix the build".to_owned()),
                last_prompt: Some("and the tests".to_owned()),
            }],
            next: None,
            already_imported: Some(2),
        };
        let text = serde_json::to_string(&page).unwrap();
        assert_eq!(
            text,
            r#"{"conversations":[{"harness_session":"past-2","cwd":"/work/app","first_prompt":"fix the build","last_prompt":"and the tests"}],"already_imported":2}"#
        );
        assert_eq!(
            serde_json::from_str::<AgentHistoryPage>(&text).unwrap(),
            page
        );
    }
}
