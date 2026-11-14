//! The timeline: a structured session's records folded into entries, as
//! the page's `data/timeline.ts` folds them.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;

use vessel_proto::{
    AgentAbilities, AgentCommand, ContextUsage, EventRecord, EventSeq, PermissionId, PlanEntry,
    PromptQueue, QuestionId, SessionEvent, SessionOption, SessionStatus, SessionTitle, ToolCallId,
    ToolLocation, TurnUsage,
};

use super::entry::{
    Entry, EntryKey, EntryKind, ParkingChange, Permission, Prompt, TextKind, ToolView,
};
use super::usage::ModelUsage;

/// What protocol 10's events say beyond the entries, for the composer and
/// the inspector (the page's `data/conversation.ts`): each event replaces
/// what it reports, so the last record is the truth.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    /// The queue as its last report left it; `None` before one.
    pub queue: Option<PromptQueue>,
    /// The commands the agent offers; `None` until it says.
    pub commands: Option<Vec<AgentCommand>>,
    /// How full the context is, from the last report.
    pub context: Option<ContextUsage>,
    /// The last turn's tokens, when the agent reported them.
    pub turn_usage: Option<TurnUsage>,
    /// What the agent can do, from its last `ready`.
    pub abilities: Option<AgentAbilities>,
}

/// What one batch changed: the screen measures again only these.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Change {
    /// The first entry the batch added; the entries from here on are new.
    pub added_from: usize,
    /// Older entries the batch changed in place, in index order, once
    /// each.
    pub changed: Vec<usize>,
}

impl Change {
    /// Whether the batch changed nothing.
    #[must_use]
    pub const fn is_empty(&self, len: usize) -> bool {
        self.added_from >= len && self.changed.is_empty()
    }
}

/// A structured session's timeline.
///
/// Entries are only ever appended or changed in place, never moved or
/// removed, so an index kept by id stays right: a tool call's update, a
/// permission's answer and a question's outcome find their entry without
/// a search, and replaying a long log is linear (the page's
/// `docs/PERFORMANCE.md`: a search per tool call made 20,000 entries
/// quadratic).
#[derive(Debug)]
pub struct Timeline {
    pub(super) entries: Vec<Entry>,
    pub(super) plan: Option<Vec<PlanEntry>>,
    /// The last plan row, which an update changing nothing a person reads
    /// refreshes in place.
    pub(super) plan_row: Option<usize>,
    pub(super) turns: u32,
    pub(super) turn: Option<EventSeq>,
    pub(super) attention: Option<EventSeq>,
    files: Vec<PathBuf>,
    touched: HashSet<PathBuf>,
    status: Option<SessionStatus>,
    usage: Option<ModelUsage>,
    options: Option<Vec<SessionOption>>,
    title: Option<SessionTitle>,
    next: EventSeq,
    pub(super) facts: Facts,
    pub(super) tools: HashMap<ToolCallId, usize>,
    pub(super) permissions: HashMap<PermissionId, usize>,
    /// The permissions still waiting, by entry index: the first is what
    /// the composer's dock asks about.
    pub(super) waiting: BTreeSet<usize>,
    pub(super) questions: HashMap<QuestionId, usize>,
    /// Entries a rewind left behind (`rewind.rs`), as index ranges.
    pub(super) rewound: Vec<std::ops::Range<usize>>,
    /// The batch being folded: where it began, and what it changed.
    pub(super) batch: Change,
}

impl Default for Timeline {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            plan: None,
            plan_row: None,
            turns: 0,
            turn: None,
            attention: None,
            files: Vec::new(),
            touched: HashSet::new(),
            status: None,
            usage: None,
            options: None,
            title: None,
            next: EventSeq::ZERO,
            facts: Facts::default(),
            tools: HashMap::new(),
            permissions: HashMap::new(),
            waiting: BTreeSet::new(),
            questions: HashMap::new(),
            rewound: Vec::new(),
            batch: Change::default(),
        }
    }
}

impl Timeline {
    /// `records` folded in, in order; records already folded (a resumed
    /// stream may overlap) are skipped by sequence number. Says which
    /// entries the batch added and which it changed.
    pub fn apply(&mut self, records: &[EventRecord]) -> Change {
        self.batch = Change {
            added_from: self.entries.len(),
            changed: Vec::new(),
        };
        for record in records {
            if record.seq < self.next {
                continue;
            }
            // `None` only once u64 is spent: nothing can follow then.
            let Some(next) = record.seq.next() else {
                break;
            };
            self.next = next;
            self.record(record);
        }
        let mut change = std::mem::take(&mut self.batch);
        change.changed.sort_unstable();
        change.changed.dedup();
        change
    }

    /// The entries, oldest first.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The latest plan.
    #[must_use]
    pub fn plan(&self) -> Option<&[PlanEntry]> {
        self.plan.as_deref()
    }

    /// Turns started so far.
    #[must_use]
    pub const fn turns(&self) -> u32 {
        self.turns
    }

    /// Whether a turn is running.
    #[must_use]
    pub const fn in_turn(&self) -> bool {
        self.turn.is_some()
    }

    /// The running turn's `turn_started` record, which Stop names
    /// (`cancel_turn { turn }`); `None` between turns.
    #[must_use]
    pub const fn turn(&self) -> Option<EventSeq> {
        self.turn
    }

    /// The last record that asks for the person's attention (a turn's end
    /// or failure, the session's end, a permission or a question), as the
    /// node counts unread; `None` before one.
    #[must_use]
    pub const fn attention(&self) -> Option<EventSeq> {
        self.attention
    }

    /// The first permission request still unanswered, in log order: what
    /// the composer's dock asks about.
    #[must_use]
    pub fn pending(&self) -> Option<(&Entry, &Permission)> {
        let index = *self.waiting.first()?;
        let entry = self.entries.get(index)?;
        match &entry.kind {
            EntryKind::Permission(permission) => Some((entry, permission)),
            EntryKind::Ready { .. }
            | EntryKind::Prompt(_)
            | EntryKind::Steer { .. }
            | EntryKind::Question { .. }
            | EntryKind::Message { .. }
            | EntryKind::Thought { .. }
            | EntryKind::Tool(_)
            | EntryKind::Plan { .. }
            | EntryKind::TurnEnded { .. }
            | EntryKind::TurnFailed { .. }
            | EntryKind::Log { .. }
            | EntryKind::Status { .. }
            | EntryKind::Parking(_)
            | EntryKind::Forked { .. }
            | EntryKind::Profile { .. }
            | EntryKind::Model(_) => None,
        }
    }

    /// How many permission requests wait (the card's "1/N").
    #[must_use]
    pub fn waiting_permissions(&self) -> usize {
        self.waiting.len()
    }

    /// Whether a permission request for tool call `id` waits.
    #[must_use]
    pub fn asks_permission(&self, id: &ToolCallId) -> bool {
        self.waiting.iter().any(|index| {
            self.entries.get(*index).is_some_and(|entry| {
                matches!(&entry.kind, EntryKind::Permission(permission) if permission.request.tool_call == *id)
            })
        })
    }

    /// The tool call `id`, when it was seen.
    #[must_use]
    pub fn tool_call(&self, id: &ToolCallId) -> Option<&ToolView> {
        let entry = self.entries.get(*self.tools.get(id)?)?;
        if let EntryKind::Tool(tool) = &entry.kind {
            Some(tool)
        } else {
            None
        }
    }

    /// The questions still waiting for an answer, oldest first.
    #[must_use]
    pub fn waiting_questions(&self) -> Vec<&Entry> {
        let mut waiting: Vec<&Entry> = self
            .questions
            .values()
            .filter_map(|index| self.entries.get(*index))
            .filter(|entry| matches!(entry.kind, EntryKind::Question { outcome: None, .. }))
            .collect();
        waiting.sort_by_key(|entry| entry.seq);
        waiting
    }

    /// Paths the session's tool calls touched, in first-touched order.
    #[must_use]
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    /// The session's status, as its records last said.
    #[must_use]
    pub const fn status(&self) -> Option<&SessionStatus> {
        self.status.as_ref()
    }

    /// Its model calls so far; `None` before the first.
    #[must_use]
    pub const fn usage(&self) -> Option<&ModelUsage> {
        self.usage.as_ref()
    }

    /// What the session runs with, as the agent last reported it
    /// (protocol 9); `None` until the node says.
    #[must_use]
    pub fn options(&self) -> Option<&[SessionOption]> {
        self.options.as_deref()
    }

    /// The title when it changed in this log; `None` when it has not (the
    /// session's listed title is then the one to show).
    #[must_use]
    pub const fn title(&self) -> Option<&SessionTitle> {
        self.title.as_ref()
    }

    /// The next record expected: where a resumed stream starts.
    #[must_use]
    pub const fn next(&self) -> EventSeq {
        self.next
    }

    /// What protocol 10's events say beyond the entries.
    #[must_use]
    pub const fn facts(&self) -> &Facts {
        &self.facts
    }

    /// The index of the entry `key` names, when it is in the log.
    #[must_use]
    pub fn index_of(&self, key: &EntryKey) -> Option<usize> {
        match key {
            EntryKey::Record(seq) => self
                .entries
                .binary_search_by_key(seq, |entry| entry.seq)
                .ok()
                .filter(|index| {
                    self.entries
                        .get(*index)
                        .is_some_and(|entry| entry.key == *key)
                }),
            EntryKey::Tool(id) => self.tools.get(id).copied(),
            EntryKey::Permission(id) => self.permissions.get(id).copied(),
            EntryKey::Question(id) => self.questions.get(id).copied(),
            EntryKey::Plan { seq, .. } => {
                let first = self.entries.partition_point(|entry| entry.seq < *seq);
                self.entries
                    .iter()
                    .enumerate()
                    .skip(first)
                    .take_while(|(_, entry)| entry.seq == *seq)
                    .find(|(_, entry)| entry.key == *key)
                    .map(|(index, _)| index)
            }
        }
    }

    fn record(&mut self, record: &EventRecord) {
        match &record.event {
            SessionEvent::Status { status, .. } => {
                self.status = Some(status.clone());
                if !status.is_live() {
                    self.turn = None;
                    self.attention = Some(record.seq);
                }
                self.push(
                    record,
                    EntryKind::Status {
                        status: status.clone(),
                    },
                );
            }
            // A parked session's agent stopped and started again (protocol
            // 12, ADR 0038): a line each, as a status change is.
            SessionEvent::Parked { reason, .. } => {
                self.push(
                    record,
                    EntryKind::Parking(ParkingChange::Parked(reason.clone())),
                );
            }
            SessionEvent::Woken => self.push(record, EntryKind::Parking(ParkingChange::Woken)),
            // A rewind leaves what followed its turn's end behind
            // (`rewind.rs`); a fork's record is its first line (`fork.rs`).
            SessionEvent::Rewound { after, .. } => self.rewound_to(*after),
            SessionEvent::Forked { from, after, .. } => self.push(
                record,
                EntryKind::Forked {
                    from: *from,
                    after: *after,
                },
            ),
            SessionEvent::Terminal(_) => {}
            SessionEvent::Agent { event, .. } => self.agent(record, event),
            SessionEvent::ProfileChanged { from, to, .. } => self.push(
                record,
                EntryKind::Profile {
                    from: from.clone(),
                    to: to.clone(),
                },
            ),
            SessionEvent::ModelCall(call) => {
                self.usage.get_or_insert_with(ModelUsage::default).add(call);
                self.push(record, EntryKind::Model(call.clone()));
            }
            // The whole set each time, so the last one is the truth.
            SessionEvent::OptionsChanged { options, .. } => self.options = Some(options.clone()),
            SessionEvent::Renamed { title, .. } => self.title = Some(title.clone()),
            SessionEvent::QueueChanged { queue, .. } => self.facts.queue = Some(queue.clone()),
            SessionEvent::CommandsChanged { commands, .. } => {
                self.facts.commands = Some(commands.clone());
            }
            SessionEvent::UsageChanged { usage, .. } => self.facts.context = Some(usage.clone()),
        }
    }

    /// A new entry made by `record`, keyed by it.
    pub(super) fn push(&mut self, record: &EventRecord, kind: EntryKind) {
        self.push_keyed(record, EntryKey::Record(record.seq), kind);
    }

    pub(super) fn push_keyed(&mut self, record: &EventRecord, key: EntryKey, kind: EntryKind) {
        self.entries.push(Entry {
            key,
            seq: record.seq,
            at: record.at,
            kind,
            rev: 0,
        });
    }

    /// The entry at `index`, to be changed in place: counted as changed.
    pub(super) fn change(&mut self, index: usize) -> Option<&mut Entry> {
        let entry = self.entries.get_mut(index)?;
        // Wrapping: the count only has to differ from the one a cache
        // kept, and four billion changes to one entry between two frames
        // do not happen.
        entry.rev = entry.rev.wrapping_add(1);
        if index < self.batch.added_from {
            self.batch.changed.push(index);
        }
        Some(entry)
    }

    /// A chunk of text joined to the last entry when it is of `kind`, else
    /// a new entry started with it (a user chunk that does not follow a
    /// prompt is a replayed prompt, of the turns so far).
    pub(super) fn merge(&mut self, record: &EventRecord, kind: TextKind, chunk: &str) {
        let last = self.entries.len().checked_sub(1);
        if let Some(index) = last
            && self
                .entries
                .get_mut(index)
                .and_then(|entry| entry.kind.text_mut(kind))
                .is_some()
        {
            if let Some(text) = self
                .change(index)
                .and_then(|entry| entry.kind.text_mut(kind))
            {
                text.push_str(chunk);
            }
            return;
        }
        let text = chunk.to_owned();
        let entry = match kind {
            TextKind::Prompt => EntryKind::Prompt(Prompt {
                text,
                replayed: true,
                turn: self.turns,
                attachments: Vec::new(),
            }),
            TextKind::Message => EntryKind::Message { text },
            TextKind::Thought => EntryKind::Thought { text },
        };
        self.push(record, entry);
    }

    /// A tool call as it now stands: its entry replaced in place, or a new
    /// one keyed by its id.
    pub(super) fn tool(&mut self, record: &EventRecord, tool: ToolView) {
        self.touch(&tool.locations);
        if let Some(index) = self.tools.get(&tool.id).copied() {
            if let Some(entry) = self.change(index) {
                entry.kind = EntryKind::Tool(tool);
            }
            return;
        }
        let _absent = self.tools.insert(tool.id.clone(), self.entries.len());
        self.push_keyed(
            record,
            EntryKey::Tool(tool.id.clone()),
            EntryKind::Tool(tool),
        );
    }

    /// The tool call `id` as it stands, if seen.
    pub(super) fn find_tool(&self, id: &ToolCallId) -> Option<&ToolView> {
        let entry = self.entries.get(*self.tools.get(id)?)?;
        match &entry.kind {
            EntryKind::Tool(tool) => Some(tool),
            EntryKind::Ready { .. }
            | EntryKind::Prompt(_)
            | EntryKind::Steer { .. }
            | EntryKind::Question { .. }
            | EntryKind::Message { .. }
            | EntryKind::Thought { .. }
            | EntryKind::Plan { .. }
            | EntryKind::Permission(_)
            | EntryKind::TurnEnded { .. }
            | EntryKind::TurnFailed { .. }
            | EntryKind::Log { .. }
            | EntryKind::Status { .. }
            | EntryKind::Parking(_)
            | EntryKind::Forked { .. }
            | EntryKind::Profile { .. }
            | EntryKind::Model(_) => None,
        }
    }

    fn touch(&mut self, locations: &[ToolLocation]) {
        for location in locations {
            if self.touched.insert(location.path.clone()) {
                self.files.push(location.path.clone());
            }
        }
    }
}
