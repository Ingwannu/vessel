//! One append/change path records exact source membership and conservative bounds.

use super::Timeline;
use super::entry::{Entry, EntryKey, EntryKind, Prompt, TextKind, ToolView};
use vessel_proto::{EventRecord, ToolCallId};

impl Timeline {
    /// A new entry made by `record`, keyed by it.
    pub(super) fn push(&mut self, record: &EventRecord, kind: EntryKind) {
        if matches!(kind, EntryKind::Prompt(_) | EntryKind::Steer { .. }) {
            self.user_stream = Some(self.entries.len());
        }
        self.push_keyed(record, EntryKey::Record(record.seq), kind);
    }

    pub(super) fn push_keyed(&mut self, record: &EventRecord, key: EntryKey, kind: EntryKind) {
        self.entries.push(Entry {
            key,
            seq: record.seq,
            through: self.next(),
            extra_sources: super::source::ExtraSources::default(),
            at: record.at,
            kind,
            rev: 0,
        });
    }

    /// The entry at `index`, to be changed in place: counted as changed.
    pub(super) fn change(&mut self, index: usize) -> Option<&mut Entry> {
        let through = self.next();
        let entry = self.entries.get_mut(index)?;
        entry.through = through;
        if let Some(source) = through
            .get()
            .checked_sub(1)
            .map(vessel_proto::EventSeq::new)
            && source != entry.seq
        {
            entry.extra_sources.record(source);
        }
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
        let last = match kind {
            TextKind::Prompt => self.user_stream,
            TextKind::Message | TextKind::Thought => self.entries.len().checked_sub(1),
        };
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
}
