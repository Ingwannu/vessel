//! Requests and snapshots are scoped to authenticated nodes and monotonically
//! increasing tickets. An offline transition invalidates every outstanding read.

use super::{Authority, Counts, Entry, Key, Pages, Position, Problem, Read, Source};
use std::collections::HashSet;
use vessel_proto::NodeId;

/// Non-repeating identity retained across navigation and reconnects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request(u64);

/// A coherent bounded read, with explicit partial state instead of pretending
/// the loaded rows describe every item on the machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Explicit operation authority from this connection.
    pub authority: Authority,
    /// Whole-node counters from the first page transaction.
    pub counts: Counts,
    /// Whole-projection session rollups; None explicitly uses legacy log marks.
    pub attention: Option<Vec<vessel_proto::inbox_attention::SessionAttention>>,
    /// Explicit terminal attention; None means the peer lacks this contract.
    pub terminals: Option<Vec<super::terminals::Row>>,
    /// Loaded rows, newest first.
    pub entries: Vec<Entry>,
    /// Continuation of the selected page, never inferred from loaded rows.
    pub next: Option<Position>,
    /// Whether older entries remain.
    pub extent: Extent,
}

/// Whether all current attention fitted the fetched page budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Extent {
    /// No further cursor was returned.
    Complete,
    /// Additional pages can be requested explicitly.
    More,
}

/// A machine's current read state. A failure retains the last good snapshot
/// separately and never claims that stale rows are current live requests.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Listing {
    /// No result yet.
    #[default]
    Loading,
    /// Read completed.
    Ready,
    /// Failure is shown beside the machine's retained snapshot.
    Failed(Problem),
}

/// One node's retained data and its current single outstanding list operation.
#[derive(Debug)]
pub struct Node {
    /// Authenticated identity plus route.
    pub source: Source,
    /// Last read status.
    pub listing: Listing,
    /// Last successful data, retained visibly stale while offline.
    pub snapshot: Option<Snapshot>,
    /// The selected cursor window, bounded independently of history depth.
    pub pages: Pages,
    pending: Option<Request>,
    hydrated: bool,
    high_water: Option<Position>,
}

/// The window's one attention store; OS delivery consumes only the new entries
/// returned when this store accepts an authoritative snapshot.
#[derive(Debug, Default)]
pub struct Inbox {
    nodes: Vec<Node>,
    serial: u64,
    poll_start: usize,
    selected: Option<Key>,
    terminal_notices: Vec<(Source, vessel_proto::notifications::TerminalAttention)>,
    terminal_coalescing: super::notices::Coalescing,
}
impl Inbox {
    /// Retained node scopes for list rendering and failure reporting.
    #[must_use]
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Choose one read admission per authenticated node, retaining the exact
    /// current one while it is still a candidate. This does not retarget an
    /// outstanding operation: reconnect admission still invalidates its ticket.
    #[must_use]
    pub fn read_routes(&self, candidates: impl IntoIterator<Item = Source>) -> Vec<Source> {
        let mut routes: Vec<Source> = Vec::new();
        for source in candidates {
            if let Some(known) = routes
                .iter_mut()
                .find(|known| known.node() == source.node())
            {
                if self.nodes.iter().any(|node| node.source == source) {
                    *known = source;
                }
            } else {
                routes.push(source);
            }
        }
        routes
    }

    /// Rotate only background admission order. A fixed first batch must not
    /// starve later machines when the link's bounded concurrency is saturated.
    /// The node list's presentation order and pending effects stay unchanged.
    #[must_use]
    pub fn poll_routes(&mut self, mut sources: Vec<Source>) -> Vec<Source> {
        if sources.is_empty() {
            self.poll_start = 0;
            return sources;
        }
        let start = self
            .poll_start
            .checked_rem(sources.len())
            .unwrap_or_default();
        sources.rotate_left(start);
        self.poll_start = start
            .checked_add(1)
            .filter(|index| *index < sources.len())
            .unwrap_or_default();
        sources
    }

    /// Expand a node's loaded prefix only between reads.
    #[must_use]
    pub fn expand(&mut self, id: NodeId) -> Option<Source> {
        let node = self
            .nodes
            .iter_mut()
            .find(|node| node.source.node() == id)?;
        if node.pending.is_some() {
            return None;
        }
        node.pages = node.pages.more(node.snapshot.as_ref()?.next?)?;
        Some(node.source.clone())
    }

    /// Return to the newest window without replaying its notice watermark.
    #[must_use]
    pub fn newest(&mut self, id: NodeId) -> Option<Source> {
        let node = self
            .nodes
            .iter_mut()
            .find(|node| node.source.node() == id)?;
        if node.pending.is_some() {
            return None;
        }
        node.pages = Pages::default();
        Some(node.source.clone())
    }

    /// Add or rename a route only for its authenticated node.
    pub fn connect(&mut self, source: Source) {
        if let Some(node) = self
            .nodes
            .iter_mut()
            .find(|node| node.source.node() == source.node())
        {
            if node.source.context() != source.context() {
                node.pending = None;
                node.hydrated = false;
                node.listing = Listing::Loading;
            }
            node.source = source;
        } else {
            self.nodes.push(Node {
                source,
                listing: Listing::Loading,
                snapshot: None,
                pages: Pages::default(),
                pending: None,
                hydrated: false,
                high_water: None,
            });
        }
    }

    /// Offline invalidates inflight reads and silences the next hydration.
    pub fn disconnect(&mut self, id: NodeId) {
        if let Some(node) = self.nodes.iter_mut().find(|node| node.source.node() == id) {
            node.pending = None;
            node.hydrated = false;
            node.listing = Listing::Failed(Problem::Offline);
        }
    }

    /// Allocate a ticket for a detail operation from the same non-repeating
    /// sequence as list reads.
    /// # Errors
    /// The counter must never wrap and reuse an earlier request identity.
    pub fn ticket(&mut self) -> Result<Request, Problem> {
        self.serial = self.serial.checked_add(1).ok_or(Problem::Exhausted)?;
        Ok(Request(self.serial))
    }

    /// Start one read for a known node; concurrent duplicate refreshes are
    /// ignored, so a slow machine cannot accumulate polling requests.
    ///
    /// # Errors
    /// The identity counter can never be reused after exhaustion.
    pub fn begin(&mut self, id: NodeId) -> Result<Option<Request>, Problem> {
        let Some(node) = self.nodes.iter_mut().find(|node| node.source.node() == id) else {
            return Ok(None);
        };
        if node.pending.is_some() {
            return Ok(None);
        }
        self.serial = self.serial.checked_add(1).ok_or(Problem::Exhausted)?;
        let ticket = Request(self.serial);
        node.pending = Some(ticket);
        Ok(Some(ticket))
    }

    /// Accept a current read only. First hydration and reconnect are silent;
    /// marking a known entry unread does not create a second OS notification.
    pub fn receive(
        &mut self,
        id: NodeId,
        ticket: Request,
        result: Result<Snapshot, Problem>,
    ) -> Vec<Key> {
        self.receive_visible(id, ticket, result).1
    }

    /// Receive a snapshot and report only changes to visible state. Polling
    /// tickets and identical heartbeats must not request a new window frame.
    pub fn receive_visible(
        &mut self,
        id: NodeId,
        ticket: Request,
        result: Result<Snapshot, Problem>,
    ) -> (bool, Vec<Key>) {
        let Some(node) = self.nodes.iter_mut().find(|node| node.source.node() == id) else {
            return (false, Vec::new());
        };
        if node.pending != Some(ticket) {
            return (false, Vec::new());
        }
        node.pending = None;
        let snapshot = match result {
            Ok(snapshot) => snapshot,
            Err(problem) => {
                let changed = node.listing != Listing::Failed(problem);
                node.listing = Listing::Failed(problem);
                return (changed, Vec::new());
            }
        };
        if !valid(id, &snapshot) {
            let changed = node.listing != Listing::Failed(Problem::InvalidReply);
            node.listing = Listing::Failed(Problem::InvalidReply);
            return (changed, Vec::new());
        }

        let changed = node.listing != Listing::Ready || node.snapshot.as_ref() != Some(&snapshot);
        let notices = fresh_notices(node, &snapshot);
        if node.hydrated
            && let Some(previous) = node.snapshot.as_ref()
        {
            for state in super::terminals::fresh(previous, &snapshot) {
                if self.terminal_notices.len() < super::notices::MAX_PENDING_NOTICES {
                    self.terminal_notices.push((node.source.clone(), state));
                } else {
                    self.terminal_coalescing = super::notices::Coalescing::Combined;
                }
            }
        }
        node.high_water = snapshot
            .entries
            .iter()
            .map(|entry| entry.position)
            .chain(node.high_water)
            .max();
        node.snapshot = Some(snapshot);
        node.hydrated = true;
        node.listing = Listing::Ready;
        (changed, notices)
    }

    /// Drain bounded terminal delivery opportunities from accepted snapshots.
    #[must_use]
    pub fn take_terminal_notices(
        &mut self,
    ) -> (
        Vec<(Source, vessel_proto::notifications::TerminalAttention)>,
        super::notices::Coalescing,
    ) {
        (
            std::mem::take(&mut self.terminal_notices),
            std::mem::take(&mut self.terminal_coalescing),
        )
    }

    /// A node-book removal discards its attention and invalidates selection.
    pub fn retain(&mut self, nodes: &[NodeId]) {
        self.nodes
            .retain(|node| nodes.contains(&node.source.node()));
        if self.selected.is_some_and(|key| !nodes.contains(&key.node)) {
            self.selected = None;
        }
    }

    /// Select an exact source; selection alone neither reads nor resolves it.
    pub fn select(&mut self, key: Key) {
        self.selected = Some(key);
    }
    /// The source the detail panel is currently confirming.
    #[must_use]
    pub const fn selected(&self) -> Option<Key> {
        self.selected
    }
    /// Current authoritative entry, if still loaded.
    #[must_use]
    pub fn entry(&self, key: Key) -> Option<&Entry> {
        self.nodes
            .iter()
            .filter(|node| node.source.node() == key.node)
            .flat_map(|node| node.snapshot.iter())
            .flat_map(|data| &data.entries)
            .find(|entry| entry.key == key)
    }
    /// Shared read marks require a current successful read. Read-only people
    /// keep this action; a retained stale/offline snapshot cannot authorize it.
    #[must_use]
    pub fn may_mark(&self, key: Key) -> bool {
        self.nodes.iter().any(|node| {
            node.source.node() == key.node
                && node.listing == Listing::Ready
                && node.snapshot.is_some()
        })
    }

    /// Answer controls require a current successful read and explicit operate
    /// authority; a retained offline snapshot never grants an action.
    #[must_use]
    pub fn may_answer(&self, key: Key) -> bool {
        self.nodes.iter().any(|node| {
            node.source.node() == key.node
                && node.listing == Listing::Ready
                && node
                    .snapshot
                    .as_ref()
                    .is_some_and(|data| data.authority == Authority::Operate)
        })
    }

    /// Aggregate whole-node counts; partial rows never lower the badge.
    #[must_use]
    pub fn counts(&self) -> Counts {
        self.nodes
            .iter()
            .filter_map(|node| node.snapshot.as_ref())
            .fold(Counts::default(), |sum, data| sum.plus(data.counts))
    }
}

fn valid(id: NodeId, snapshot: &Snapshot) -> bool {
    let mut seen = HashSet::new();
    snapshot
        .entries
        .iter()
        .all(|entry| entry.key.node == id && seen.insert(entry.key))
        && {
            let mut terminals = HashSet::new();
            snapshot.terminals.iter().flatten().all(|row| {
                terminals.insert(row.terminal)
                    && row
                        .attention
                        .is_none_or(|state| state.terminal == row.terminal)
            })
        }
}

fn fresh_notices(node: &Node, snapshot: &Snapshot) -> Vec<Key> {
    if !node.hydrated {
        return Vec::new();
    }
    snapshot
        .entries
        .iter()
        .filter(|entry| {
            entry.read == Read::Unread
                && entry.content.announces()
                && node
                    .high_water
                    .is_none_or(|position| entry.position > position)
        })
        .map(|entry| entry.key)
        .collect()
}
