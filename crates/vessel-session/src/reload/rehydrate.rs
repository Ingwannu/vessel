//! What a reload adds to the log from the agent's own history (TRACKER
//! 803, ADR 0110, the coordinator's ruling C "R2"): the replay `session/load`
//! sends is gathered within bounds, and only what lies beyond the log's
//! prompts is added.
//!
//! [Decision Log]
//! - Bounds (the ruling: time, items and bytes for the gather, a cap for
//!   what is added): [`GATHER_TIME`] after the reload starts,
//!   [`GATHER_ITEMS`] replayed events, and [`GATHER_BYTES`] of their
//!   serialized size. Past any of them the gather drops what it held, keeps
//!   nothing more, and the reload adds nothing, saying why (as for a history
//!   that does not line up): half a history cannot be lined up.
//! - Why bytes as well as items: a count alone lets a few huge records (a
//!   tool's whole output, a large diff) hold the node's memory; the
//!   serialized size is what the log would store. It is counted without
//!   keeping the bytes, so weighing an event costs one serialization and no
//!   copy. Considered: summing the text of each event's blocks (cheaper, but
//!   misses raw tool input and output, which are the large ones), and a
//!   larger item count only (no bound on memory). Cost: each kept event is
//!   serialized once more during a reload.
//!   [`APPEND_MAX`] records are added at most, in whole user messages; a
//!   later reload adds the rest, since the log's prompts then still begin
//!   the history.
//! - Kept: the conversation's own events (user and agent messages,
//!   thoughts, tool calls and their updates, plans). The handshake's
//!   `ready` is consumed as a wake's is; stderr lines are published as they
//!   come; turn ends, permissions and questions are not part of a replay.
//! - Lined up: the log's prompts must be exactly the first user messages
//!   of the history, in order and as text (`prompts`). The records from the
//!   next user message on are added; none when there is no next one.

use std::time::Duration;

use tokio::time::Instant;
use vessel_proto::AgentEvent;

use super::prompts::{UserPrompts, messages};

/// How long after a reload starts its replay is gathered.
pub(crate) const GATHER_TIME: Duration = Duration::from_secs(60);
/// How many replayed events are gathered at most.
pub(crate) const GATHER_ITEMS: usize = 20_000;
/// How large the gathered events are at most, serialized (64 MiB).
pub(crate) const GATHER_BYTES: usize = 64 * 1024 * 1024;
/// How many records one reload adds at most.
pub(crate) const APPEND_MAX: usize = 2_000;

/// A replay as it arrives, within its bounds.
#[derive(Debug)]
pub(crate) struct Gather {
    events: Vec<AgentEvent>,
    /// The serialized size of `events`.
    bytes: usize,
    until: Instant,
    over: Option<Over>,
}

/// Which bound a gather passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Over {
    Time,
    Items,
    Bytes,
}

impl Gather {
    /// A gather that ends [`GATHER_TIME`] from now.
    pub(crate) fn new() -> Self {
        let now = Instant::now();
        Self {
            events: Vec::new(),
            bytes: 0,
            until: now.checked_add(GATHER_TIME).unwrap_or(now),
            over: None,
        }
    }

    /// One replayed event.
    pub(crate) fn push(&mut self, event: AgentEvent) {
        if self.over.is_some() || !kept(&event) {
            return;
        }
        let bytes = self.bytes.saturating_add(weight(&event));
        let over = if Instant::now() > self.until {
            Some(Over::Time)
        } else if self.events.len() >= GATHER_ITEMS {
            Some(Over::Items)
        } else if bytes > GATHER_BYTES {
            Some(Over::Bytes)
        } else {
            None
        };
        if over.is_some() {
            self.over = over;
            self.events = Vec::new();
            self.bytes = 0;
            return;
        }
        self.bytes = bytes;
        self.events.push(event);
    }
}

/// An event's serialized size, counted without keeping the bytes.
fn weight(event: &AgentEvent) -> usize {
    let mut counter = Counter(0);
    // Writing to a counter does not fail; an event that cannot be
    // serialized weighs what was written before the error.
    let _written = serde_json::to_writer(&mut counter, event);
    counter.0
}

/// A writer that only counts.
struct Counter(usize);

impl std::io::Write for Counter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0 = self.0.saturating_add(buf.len());
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Whether a replayed event is part of the conversation a reload adds.
const fn kept(event: &AgentEvent) -> bool {
    match event {
        AgentEvent::UserMessageChunk { .. }
        | AgentEvent::MessageChunk { .. }
        | AgentEvent::ThoughtChunk { .. }
        | AgentEvent::ToolCall(_)
        | AgentEvent::ToolCallUpdate(_)
        | AgentEvent::Plan { .. } => true,
        AgentEvent::Ready { .. }
        | AgentEvent::HarnessLog { .. }
        | AgentEvent::TurnStarted { .. }
        | AgentEvent::TurnEnded { .. }
        | AgentEvent::TurnFailed { .. }
        | AgentEvent::PermissionRequested(_)
        | AgentEvent::PermissionResolved { .. }
        | AgentEvent::QuestionAsked(_)
        | AgentEvent::QuestionResolved { .. }
        | AgentEvent::Steered { .. } => false,
    }
}

/// What the agent's history adds to the log.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// The log has every user message the history has.
    Current,
    /// These records follow `reloaded`; `more` when whole messages past
    /// [`APPEND_MAX`] wait for another reload.
    Append {
        records: Vec<AgentEvent>,
        more: bool,
    },
    /// Nothing is added; the log gets this line.
    Nothing(&'static str),
}

/// The line when the log's prompts do not begin the history.
pub(crate) const DIVERGED: &str = "vessel: the agent's own history does not begin with this log's \
                                   prompts, so nothing of it was added";
/// The line when the log's earlier prompts are no longer kept.
pub(crate) const UNKNOWN: &str = "vessel: this log's earlier prompts are no longer kept, so the \
                                  agent's own history was not compared and nothing of it was added";
const TOO_SLOW: &str = "vessel: the agent's own history took longer than 60 seconds to load, so \
                        nothing of it was added";
const TOO_MANY: &str = "vessel: the agent's own history is longer than 20000 records, so nothing \
                        of it was added";
const TOO_BIG: &str = "vessel: the agent's own history is larger than 64 MiB, so nothing of it \
                       was added";
const TOO_LONG: &str = "vessel: the agent's next message in its own history is longer than 2000 \
                        records, so nothing of it was added";
/// The line when part of what lies beyond the log waits for another
/// reload.
pub(crate) const MORE: &str = "vessel: more of the agent's own history remains; reload again to \
                               add it";

/// What `gather` adds to a log whose user prompts are `log`.
pub(crate) fn outcome(log: &UserPrompts, gather: Gather) -> Outcome {
    match gather.over {
        Some(Over::Time) => return Outcome::Nothing(TOO_SLOW),
        Some(Over::Items) => return Outcome::Nothing(TOO_MANY),
        Some(Over::Bytes) => return Outcome::Nothing(TOO_BIG),
        None => {}
    }
    let Some(logged) = log.prompts() else {
        return Outcome::Nothing(UNKNOWN);
    };
    let history = messages(&gather.events);
    let lined_up = logged.len() <= history.len()
        && logged
            .iter()
            .zip(&history)
            .all(|(said, (_, kept))| *said == kept.as_str());
    if !lined_up {
        return Outcome::Nothing(DIVERGED);
    }
    let beyond = history.get(logged.len()..).unwrap_or_default();
    let Some(&(from, _)) = beyond.first() else {
        return Outcome::Current;
    };
    let end = gather.events.len();
    // Whole messages from `from`, while they fit under the cap.
    let mut cut = from;
    for (index, _) in beyond.iter().enumerate() {
        let next = beyond
            .get(index.saturating_add(1))
            .map_or(end, |&(at, _)| at);
        if next.saturating_sub(from) > APPEND_MAX {
            break;
        }
        cut = next;
    }
    if cut == from {
        return Outcome::Nothing(TOO_LONG);
    }
    let mut events = gather.events;
    events.truncate(cut);
    Outcome::Append {
        records: events.split_off(from),
        more: cut < end,
    }
}
