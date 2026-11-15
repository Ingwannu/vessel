//! Records written as the page's tests write them: the wire's JSON, so a
//! case reads the same on both sides.

use serde_json::{Value, json};
use vessel_app_view::timeline::{Entry, EntryKey, Timeline};
use vessel_app_view::{ConversationWords, Lang};

/// The English table (the page's words).
pub(crate) const EN: &ConversationWords = Lang::En.conversation();

/// The Korean table.
pub(crate) const KO: &ConversationWords = Lang::Ko.conversation();
use vessel_proto::EventRecord;

/// The time every record of a case is written at.
pub(crate) const AT: &str = "2026-11-07T12:00:00Z";

/// A log being written, one record after another.
#[derive(Debug, Default)]
pub(crate) struct Log {
    pub(crate) seq: u64,
}

impl Log {
    /// A record of `event` (a session event's JSON), at the next seq.
    pub(crate) fn record(&mut self, event: &Value) -> EventRecord {
        let record = serde_json::from_value(json!({ "seq": self.seq, "at": AT, "event": event }))
            .expect("a session event the wire takes");
        self.seq = self.seq.saturating_add(1);
        record
    }

    /// A record of an agent event.
    pub(crate) fn agent(&mut self, event: &Value) -> EventRecord {
        self.record(&json!({ "type": "agent", "event": event }))
    }

    /// A model call's record.
    pub(crate) fn model(&mut self, call: Value) -> EventRecord {
        let mut event = call;
        event["type"] = json!("model_call");
        self.record(&event)
    }
}

/// A node-minted id (permissions, questions are UUIDs on the wire), by a
/// number: the page's fixtures name them `perm-1`, here they are numbered.
pub(crate) fn uuid(n: u32) -> String {
    format!("00000000-0000-4000-8000-{n:012}")
}

/// A timeline with `records` folded in.
pub(crate) fn folded(records: &[EventRecord]) -> Timeline {
    let mut timeline = Timeline::default();
    let _change = timeline.apply(records);
    timeline
}

/// The page's name for an entry's kind.
pub(crate) fn kind(entry: &Entry) -> &'static str {
    use vessel_app_view::timeline::EntryKind as K;
    match &entry.kind {
        K::Ready { .. } => "ready",
        K::Prompt(_) => "prompt",
        K::Steer { .. } => "steer",
        K::Question { .. } => "question",
        K::Message { .. } => "message",
        K::Thought { .. } => "thought",
        K::Tool(_) => "tool",
        K::Plan { .. } => "plan",
        K::Permission(_) => "permission",
        K::TurnEnded { .. } => "turn_ended",
        K::TurnFailed { .. } => "turn_failed",
        K::Log { .. } => "log",
        K::Status { .. } => "status",
        K::Parking(_) => "parking",
        K::Forked { .. } => "forked",
        K::Profile { .. } => "profile",
        K::Model(_) => "model",
    }
}

/// The kinds of every entry, in order.
pub(crate) fn kinds(timeline: &Timeline) -> Vec<&'static str> {
    timeline.entries().iter().map(kind).collect()
}

/// The page's key for an entry: `r12`, `tcall-1`, `p<uuid>`, `q<uuid>`.
pub(crate) fn key(entry: &Entry) -> String {
    match &entry.key {
        EntryKey::Record(seq) => format!("r{seq}"),
        EntryKey::Tool(id) => format!("t{id}"),
        EntryKey::Permission(id) => format!("p{id}"),
        EntryKey::Question(id) => format!("q{id}"),
        EntryKey::Plan { seq, step } => format!("r{seq}.{step}"),
    }
}

/// Entries made directly, as the page's tests make them.
pub(crate) mod make {
    use chrono::DateTime;
    use vessel_app_view::timeline::{Entry, EntryKey, EntryKind, Prompt, ToolView};
    use vessel_proto::{EventSeq, ToolCallId, ToolCallStatus, ToolKind};

    fn entry(seq: u64, kind: EntryKind) -> Entry {
        Entry {
            key: EntryKey::Record(EventSeq::new(seq)),
            seq: EventSeq::new(seq),
            at: DateTime::UNIX_EPOCH,
            kind,
            rev: 0,
        }
    }

    pub(crate) fn prompt(seq: u64, text: &str) -> Entry {
        entry(
            seq,
            EntryKind::Prompt(Prompt {
                text: text.to_owned(),
                replayed: false,
                turn: 1,
                attachments: Vec::new(),
            }),
        )
    }

    pub(crate) fn message(seq: u64, text: &str) -> Entry {
        entry(
            seq,
            EntryKind::Message {
                text: text.to_owned(),
            },
        )
    }

    pub(crate) fn thought(seq: u64) -> Entry {
        entry(
            seq,
            EntryKind::Thought {
                text: "…".to_owned(),
            },
        )
    }

    pub(crate) fn log(seq: u64, line: &str) -> Entry {
        entry(
            seq,
            EntryKind::Log {
                line: line.to_owned(),
            },
        )
    }

    pub(crate) fn tool(seq: u64, kind: ToolKind) -> Entry {
        let id = ToolCallId::new(format!("c{seq}"));
        Entry {
            key: EntryKey::Tool(id.clone()),
            ..entry(
                seq,
                EntryKind::Tool(ToolView {
                    id,
                    title: "Read".to_owned(),
                    kind,
                    status: ToolCallStatus::Completed,
                    content: Vec::new(),
                    locations: Vec::new(),
                    input: None,
                    output: None,
                }),
            )
        }
    }

    /// A conversation as `fake-acp`'s `entries` makes it: a message, a
    /// thought and a tool call in turn, after one prompt.
    pub(crate) fn conversation(count: u64) -> Vec<Entry> {
        let mut entries = vec![prompt(0, "go")];
        for index in 0..count {
            let seq = index.saturating_add(1);
            entries.push(match index % 3 {
                0 => message(seq, &format!("Entry {index} names module_{index}.rs")),
                1 => thought(seq),
                _ => tool(seq, ToolKind::Read),
            });
        }
        entries
    }
}
