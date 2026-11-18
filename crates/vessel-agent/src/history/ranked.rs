//! A ranked listing (ADR 0109, partly superseding ADR 0053): the newest
//! `limit` conversations an agent keeps that the node does not hold yet,
//! each with its first and last prompt, as Paseo's import sheet lists them
//! (`listImportableProviderSessions`, the ACP provider's
//! `listImportableSessions`).
//!
//! [Decision Log]
//! - Purpose: Paseo's sheet shows a prompt preview under every row,
//!   searches on the server, pages by asking for a larger `limit` and says
//!   "All recent sessions are already imported." from a count of what it
//!   left out. ADR 0053's cursor pages had none of these, so clients
//!   filtered pages they had read and queued their reads.
//! - Constraint: an ACP agent's `session/list` gives titles and times
//!   only; the prompts are in the conversation, which only `session/load`
//!   replays. A load is seconds for some adapters, so it is bounded.
//! - Alternatives: reading each harness's files for the prompts (refused
//!   in ADR 0053: breaks with every format change, reads another program's
//!   files); no previews (ADR 0053; the sheet's rows then differ from
//!   Paseo's); loading only the rows answered with no budget (an adapter
//!   that hangs on one load would hold the whole listing).
//! - Choice: Paseo's: one listing process; its pages read in the agent's
//!   order; every conversation not already on the node is loaded on the
//!   same connection, no MCP servers and no prompt, its replayed user
//!   messages gathered (`Gathered`), then closed when the agent can
//!   (`session/close`, at most [`CLOSE_LIMIT`]). Loads stop at
//!   [`PREVIEW_BUDGET`] from the first page, each at most [`LOAD_LIMIT`];
//!   past them a row is kept without a preview. A conversation whose
//!   replay has nothing in it is left out (Paseo's `hasConversation`). A
//!   preview is kept per account and conversation while its time
//!   (`updated_at`) is unchanged ([`Previews`]), so a conversation just
//!   had is read again. The listing ends with `limit` rows or after
//!   [`SCAN`] of the agent's conversations.
//! - Why not a fresh agent run per conversation: a load on the listing
//!   connection replays only; nothing is prompted, and the listing client
//!   answers no tool, file, terminal or permission request, so the agent
//!   cannot act through it (coordinator's condition, 2026-11-18). An agent
//!   whose load fails or times out keeps the row without a preview.
//! - Impact: a ranked listing costs one agent start plus a load per row
//!   answered (seconds each, cached after), up to [`RANKED_LIMIT`] in all;
//!   the node bounds how many run at once.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use agent_client_protocol::v1::schema as acp;
use tokio::time::Instant;
use vessel_proto::HarnessSessionId;

use super::HistoryEntry;

/// How long a listing loads conversations for their prompts, from its
/// first page (Paseo's `ACP_IMPORT_HISTORY_BUDGET_MS`).
pub const PREVIEW_BUDGET: Duration = Duration::from_secs(60);

/// How long one conversation's load may take (Paseo's
/// `ACP_IMPORT_HISTORY_LOAD_TIMEOUT_MS`), and never past the budget.
pub const LOAD_LIMIT: Duration = Duration::from_secs(30);

/// How long closing a loaded conversation may take (Paseo's
/// `ACP_PROBE_CLOSE_TIMEOUT_MS`), and never past the budget.
pub const CLOSE_LIMIT: Duration = Duration::from_secs(2);

/// A whole ranked listing, the agent's start included (Paseo's
/// `IMPORTABLE_SESSION_LIST_TIMEOUT_MS`).
pub const RANKED_LIMIT: Duration = Duration::from_secs(90);

/// The most of the agent's conversations one listing looks through
/// (Paseo's `IMPORT_SESSION_SEARCH_SCAN_LIMIT`).
pub const SCAN: usize = 500;

/// The longest prompt preview, in characters (Paseo's 160).
pub const PREVIEW_CHARS: usize = 160;

/// The most consecutive pages without a conversation a listing asks past.
const EMPTY_PAGES: usize = 10;

/// What a ranked listing asks.
#[derive(Debug, Clone)]
pub struct Ranked {
    /// How many conversations to answer.
    pub limit: usize,
    /// What they must hold, trimmed and in lower case; `None` for all.
    query: Option<String>,
    /// The conversations already on the node, left out and counted.
    pub known: HashSet<HarnessSessionId>,
    /// Prompts already read, kept by the node across listings.
    pub previews: Previews,
    /// Whose conversations these are (harness and profile), so one
    /// account's previews never answer another's.
    pub account: String,
}

impl Ranked {
    /// A listing of `limit` conversations holding `query` (any without).
    #[must_use]
    pub fn new(limit: usize, query: Option<&str>, previews: Previews, account: String) -> Self {
        let query = query
            .map(|query| query.trim().to_lowercase())
            .filter(|query| !query.is_empty());
        Self {
            limit,
            query,
            known: HashSet::new(),
            previews,
            account,
        }
    }

    /// The same, leaving out `known` (counted in `already_imported`).
    #[must_use]
    pub fn without(mut self, known: HashSet<HarnessSessionId>) -> Self {
        self.known = known;
        self
    }

    /// The query as matched, if any.
    #[must_use]
    pub fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }
}

/// A ranked listing's answer.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RankedPage {
    /// At most `limit`, in the agent's order (the node sorts them by time).
    pub entries: Vec<HistoryEntry>,
    /// How many of those looked through (and matching the query) were
    /// already on the node.
    pub already_imported: usize,
}

/// A conversation's prompts, as its load replayed them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Prompts {
    /// Its first prompt, folded and cut ([`preview`]).
    pub first: Option<String>,
    /// Its last prompt, the same.
    pub last: Option<String>,
    /// Whether the replay held anything of a conversation (messages,
    /// thoughts, tool calls, a plan); one without is left out.
    pub said: bool,
}

/// The prompts read so far, per account and conversation, while the
/// conversation's time is the one they were read at. Shared by the node's
/// listings; at most [`Previews::KEPT`] conversations, the oldest read
/// forgotten first.
#[derive(Debug, Clone, Default)]
pub struct Previews(Arc<Mutex<Kept>>);

#[derive(Debug, Default)]
struct Kept {
    by: HashMap<(String, HarnessSessionId), (String, Prompts)>,
    order: VecDeque<(String, HarnessSessionId)>,
}

impl Previews {
    /// The most conversations kept.
    pub const KEPT: usize = 4096;

    /// The prompts of `entry` in `account`, read while it had its time.
    #[must_use]
    pub fn get(&self, account: &str, entry: &HistoryEntry) -> Option<Prompts> {
        let at = entry.updated_at.as_deref()?;
        let kept = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let (read_at, prompts) = kept.by.get(&(account.to_owned(), entry.session.clone()))?;
        (read_at == at).then(|| prompts.clone())
    }

    /// Keep `prompts` for `entry` in `account`; nothing for a conversation
    /// without a time (it could not be told apart from a later one).
    pub fn put(&self, account: &str, entry: &HistoryEntry, prompts: Prompts) {
        let Some(at) = entry.updated_at.clone() else {
            return;
        };
        let key = (account.to_owned(), entry.session.clone());
        let mut kept = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if kept.by.insert(key.clone(), (at, prompts)).is_none() {
            kept.order.push_back(key);
            while kept.order.len() > Self::KEPT {
                if let Some(oldest) = kept.order.pop_front() {
                    let _forgotten = kept.by.remove(&oldest);
                }
            }
        }
    }
}

/// Where a ranked listing reads: the agent's pages, and each
/// conversation's load. Over ACP in [`super`]; in the tests a script with
/// paused time.
pub(crate) trait Source {
    /// The agent's page `cursor` names (its first without): its
    /// conversations with an absolute folder, and the next page's cursor.
    async fn page(
        &mut self,
        cursor: Option<String>,
    ) -> Result<(Vec<HistoryEntry>, Option<String>), super::Unlisted>;

    /// Load `entry` and say what its replay held; `None` when the agent
    /// refused or failed.
    async fn load(&mut self, entry: &HistoryEntry) -> Option<Prompts>;

    /// Stop gathering `entry`'s replay (after its load, or a load given
    /// up on).
    fn forget(&mut self, entry: &HistoryEntry);

    /// Close `entry` after its load, when the agent can.
    async fn close(&mut self, entry: &HistoryEntry);
}

/// The ranked listing over `source` (see the module's Decision Log).
pub(crate) async fn rank<S: Source>(
    source: &mut S,
    ask: &Ranked,
) -> Result<RankedPage, super::Unlisted> {
    // Unrepresentable only past the clock's end: then nothing is loaded.
    let deadline = Instant::now()
        .checked_add(PREVIEW_BUDGET)
        .unwrap_or_else(Instant::now);
    let mut answer = RankedPage::default();
    let mut scanned: usize = 0;
    let mut empty: usize = 0;
    let mut cursor = None;
    loop {
        let (entries, next) = source.page(cursor.take()).await?;
        empty = if entries.is_empty() {
            empty.saturating_add(1)
        } else {
            0
        };
        for entry in entries {
            if scanned >= SCAN || answer.entries.len() >= ask.limit {
                return Ok(answer);
            }
            scanned = scanned.saturating_add(1);
            if ask.known.contains(&entry.session) {
                // Not loaded: it is not answered. A preview read before
                // still counts toward the query, as Paseo's would.
                let kept = ask.previews.get(&ask.account, &entry);
                if holds(ask.query(), &entry, kept.as_ref()) {
                    answer.already_imported = answer.already_imported.saturating_add(1);
                }
                continue;
            }
            let prompts = prompts(source, ask, &entry, deadline).await;
            if prompts.as_ref().is_some_and(|prompts| !prompts.said) {
                continue;
            }
            if !holds(ask.query(), &entry, prompts.as_ref()) {
                continue;
            }
            let (first_prompt, last_prompt) = prompts.map_or((None, None), |p| (p.first, p.last));
            answer.entries.push(HistoryEntry {
                first_prompt,
                last_prompt,
                ..entry
            });
        }
        match next {
            Some(next)
                if empty < EMPTY_PAGES && scanned < SCAN && answer.entries.len() < ask.limit =>
            {
                cursor = Some(next);
            }
            _ => return Ok(answer),
        }
    }
}

/// A driver's listing that cannot load, ranked the same way: its
/// conversations without prompts (kept ones aside), the known ones left
/// out and counted, the query matched on titles and folders.
pub(crate) fn unloaded(entries: Vec<HistoryEntry>, ask: &Ranked) -> RankedPage {
    let mut answer = RankedPage::default();
    for entry in entries.into_iter().take(SCAN) {
        if answer.entries.len() >= ask.limit {
            break;
        }
        let kept = ask.previews.get(&ask.account, &entry);
        if !holds(ask.query(), &entry, kept.as_ref()) {
            continue;
        }
        if ask.known.contains(&entry.session) {
            answer.already_imported = answer.already_imported.saturating_add(1);
        } else {
            answer.entries.push(entry);
        }
    }
    answer
}

/// `entry`'s prompts: kept from before, or loaded within the budget.
async fn prompts<S: Source>(
    source: &mut S,
    ask: &Ranked,
    entry: &HistoryEntry,
    deadline: Instant,
) -> Option<Prompts> {
    if let Some(kept) = ask.previews.get(&ask.account, entry) {
        return Some(kept);
    }
    let left = deadline.saturating_duration_since(Instant::now());
    if left.is_zero() {
        return None;
    }
    let loaded = tokio::time::timeout(left.min(LOAD_LIMIT), source.load(entry))
        .await
        .ok()
        .flatten();
    source.forget(entry);
    let left = deadline.saturating_duration_since(Instant::now());
    if !left.is_zero() {
        // A close that fails or is slow leaves the conversation loaded in
        // the listing process, which is stopped after anyway.
        let _closed = tokio::time::timeout(left.min(CLOSE_LIMIT), source.close(entry)).await;
    }
    if let Some(loaded) = &loaded {
        ask.previews.put(&ask.account, entry, loaded.clone());
    }
    loaded
}

/// Whether `entry` holds `query` in its title, a prompt or its folder's
/// last name, letter case aside (Paseo's `matchesImportableSessionQuery`).
fn holds(query: Option<&str>, entry: &HistoryEntry, prompts: Option<&Prompts>) -> bool {
    let Some(query) = query else {
        return true;
    };
    let folder = entry
        .cwd
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    [
        entry.title.as_deref(),
        prompts.and_then(|prompts| prompts.first.as_deref()),
        prompts.and_then(|prompts| prompts.last.as_deref()),
        folder.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|text| text.to_lowercase().contains(query))
}

/// A prompt as a preview: blanks folded to one space, at most
/// [`PREVIEW_CHARS`] characters; `None` when nothing is left.
#[must_use]
pub fn preview(text: &str) -> Option<String> {
    let folded = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if folded.is_empty() {
        return None;
    }
    Some(folded.chars().take(PREVIEW_CHARS).collect())
}

/// The replays being gathered on a listing connection, by conversation
/// (Paseo's `ACPImportHistoryCollector`): its user messages, one per
/// message id, and whether anything of a conversation came.
#[derive(Debug, Default)]
pub(crate) struct Gathered {
    open: HashMap<String, Gathering>,
}

#[derive(Debug, Default)]
struct Gathering {
    prompts: Vec<String>,
    current: Option<(Option<String>, String)>,
    said: bool,
}

impl Gathering {
    fn flush(&mut self) {
        if let Some((_id, text)) = self.current.take()
            && !text.trim().is_empty()
        {
            self.prompts.push(text);
        }
    }
}

impl Gathered {
    /// Start gathering `session`'s replay.
    pub(crate) fn begin(&mut self, session: &str) {
        let _earlier = self.open.insert(session.to_owned(), Gathering::default());
    }

    /// One notification from the agent; ignored unless its conversation is
    /// being gathered.
    pub(crate) fn accept(&mut self, notification: &acp::SessionNotification) {
        let Some(gathering) = self.open.get_mut(&*notification.session_id.0) else {
            return;
        };
        let update = &notification.update;
        if matches!(
            update,
            acp::SessionUpdate::UserMessageChunk(_)
                | acp::SessionUpdate::AgentMessageChunk(_)
                | acp::SessionUpdate::AgentThoughtChunk(_)
                | acp::SessionUpdate::ToolCall(_)
                | acp::SessionUpdate::ToolCallUpdate(_)
                | acp::SessionUpdate::Plan(_)
        ) {
            gathering.said = true;
        }
        let acp::SessionUpdate::UserMessageChunk(chunk) = update else {
            gathering.flush();
            return;
        };
        let text = block_text(&chunk.content);
        if text.is_empty() {
            return;
        }
        let id = chunk.message_id.as_ref().map(|id| String::from(&*id.0));
        if let Some((Some(current), _)) = &gathering.current
            && id.as_ref() != Some(current)
        {
            gathering.flush();
        }
        gathering
            .current
            .get_or_insert_with(|| (id, String::new()))
            .1
            .push_str(&text);
    }

    /// What `session`'s replay held; it is no longer gathered.
    pub(crate) fn finish(&mut self, session: &str) -> Prompts {
        let mut gathering = self.open.remove(session).unwrap_or_default();
        gathering.flush();
        Prompts {
            first: gathering.prompts.first().and_then(|text| preview(text)),
            last: gathering.prompts.last().and_then(|text| preview(text)),
            said: gathering.said,
        }
    }

    /// Stop gathering `session` without an answer.
    pub(crate) fn forget(&mut self, session: &str) {
        let _dropped = self.open.remove(session);
    }
}

/// A content block as a prompt's words (Paseo's `contentBlockToText`):
/// text as it is, a link by its title or address, an embedded text by its
/// text, other media by a placeholder; a kind ACP adds later by nothing.
fn block_text(block: &acp::ContentBlock) -> String {
    if let acp::ContentBlock::Text(text) = block {
        return text.text.clone();
    }
    if let acp::ContentBlock::ResourceLink(link) = block {
        return link.title.clone().unwrap_or_else(|| link.uri.clone());
    }
    if let acp::ContentBlock::Resource(embedded) = block {
        if let acp::EmbeddedResourceResource::TextResourceContents(text) = &embedded.resource {
            return text.text.clone();
        }
        if let acp::EmbeddedResourceResource::BlobResourceContents(blob) = &embedded.resource {
            let kind = blob.mime_type.as_deref().unwrap_or("binary");
            return format!("[resource:{kind}]");
        }
        return String::new();
    }
    if matches!(block, acp::ContentBlock::Image(_)) {
        return "[image]".to_owned();
    }
    if matches!(block, acp::ContentBlock::Audio(_)) {
        return "[audio]".to_owned();
    }
    String::new()
}

#[cfg(test)]
mod tests;
