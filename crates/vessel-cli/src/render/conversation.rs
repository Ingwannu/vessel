//! Protocol 10's conversation as text (Track A): the queue, the agent's
//! commands and context, and its questions, one line or block each, for
//! `vessel attach` and `vessel log`; the queue entry by entry, what became
//! of a prompt and a session's marks, for the `vessel session` commands.

use std::fmt::Write as _;

use vessel_proto::conversation::{AnswerValue, FieldKind};
use vessel_proto::{
    AgentCommand, ContentBlock, ContextUsage, Creator, Delivered, PromptQueue, Question,
    QuestionOutcome, QueueHold, QueuedPrompt, SessionMarks,
};

/// How many commands a line names before it counts the rest.
const COMMANDS_SHOWN: usize = 12;
/// How many characters of a waiting prompt's first line are shown.
const PROMPT_SHOWN: usize = 60;

/// The queue in a line: how many wait, and why it is held if it is.
pub(crate) fn queue(queue: &PromptQueue) -> String {
    let held = queue.held.map(|hold| match hold {
        QueueHold::Cancelled => "held: a turn was cancelled",
        QueueHold::Failed => "held: a turn failed",
        QueueHold::Ended => "held: the session ended",
        QueueHold::Asked => "held",
    });
    match held {
        Some(why) => format!("[queue: {} waiting, {why}]", queue.entries.len()),
        None => format!("[queue: {} waiting]", queue.entries.len()),
    }
}

/// The queue, then each entry on a line: its place, id and revision, who
/// queued it, the start of its text and how many files go with it.
pub(crate) fn queue_entries(waiting: &PromptQueue) -> String {
    let mut text = queue(waiting);
    text.push('\n');
    for (place, entry) in (1_usize..).zip(&waiting.entries) {
        let _infallible = writeln!(
            text,
            "{place:>2}  {}  r{}  {:<14} {}",
            entry.id,
            entry.revision,
            creator(&entry.by),
            prompt(entry)
        );
    }
    text
}

/// The start of a waiting prompt's text, and its attachments' count.
fn prompt(entry: &QueuedPrompt) -> String {
    let mut words = String::new();
    let mut files = 0_usize;
    for block in &entry.content {
        match block {
            ContentBlock::Text { text } if words.is_empty() => {
                text.lines()
                    .next()
                    .unwrap_or_default()
                    .clone_into(&mut words);
            }
            ContentBlock::Attachment { .. } => files = files.saturating_add(1),
            ContentBlock::Text { .. }
            | ContentBlock::ResourceLink { .. }
            | ContentBlock::Opaque { .. } => {}
        }
    }
    let mut shown: String = words.chars().take(PROMPT_SHOWN).collect();
    if shown.len() < words.len() {
        shown.push('…');
    }
    match files {
        0 => shown,
        1 => format!("{shown} (+1 file)"),
        files => format!("{shown} (+{files} files)"),
    }
}

/// Who did something, in a word or two.
pub(crate) fn creator(creator: &Creator) -> String {
    match creator {
        Creator::Owner => "owner".to_owned(),
        Creator::Device { id } => format!("device {}", short(&id.to_string())),
        Creator::Agent { session, .. } => format!("agent {}", short(&session.to_string())),
    }
}

/// An id's first eight characters, enough to recognise it.
fn short(id: &str) -> String {
    id.chars().take(8).collect()
}

/// What became of a prompt.
pub(crate) fn delivered(delivered: &Delivered) -> String {
    match delivered {
        Delivered::Started => "started".to_owned(),
        Delivered::Steered => "steered".to_owned(),
        Delivered::Queued { entry } => format!("queued {entry}"),
    }
}

/// A session's marks, comma-separated; empty when it has none.
pub(crate) fn marks(marks: &SessionMarks) -> String {
    let mut words = Vec::new();
    if marks.pinned {
        words.push("pinned".to_owned());
    }
    if let Some(when) = marks.archived_at {
        words.push(format!("archived {}", when.format("%Y-%m-%d %H:%M")));
    }
    if marks.unread {
        words.push("unread".to_owned());
    }
    words.join(", ")
}

/// The commands' names, the first few of them.
pub(crate) fn commands(commands: &[AgentCommand]) -> String {
    if commands.is_empty() {
        return "none".to_owned();
    }
    let mut text = commands
        .iter()
        .take(COMMANDS_SHOWN)
        .map(|command| format!("/{}", command.name))
        .collect::<Vec<_>>()
        .join(" ");
    let rest = commands.len().saturating_sub(COMMANDS_SHOWN);
    if rest > 0 {
        let _infallible = write!(text, " and {rest} more");
    }
    text
}

/// How full the context is, with the cost when the agent says one.
pub(crate) fn usage(usage: &ContextUsage) -> String {
    let mut text = format!(
        "[context: {} of {} tokens",
        usage.used.get(),
        usage.size.get()
    );
    if let Some(cost) = &usage.cost {
        let _infallible = write!(text, ", {} {}", cost.amount.get(), cost.unit);
    }
    text.push(']');
    text
}

/// A question and its form, field by field.
pub(crate) fn question(question: &Question) -> String {
    let mut text = question_form(question);
    text.push_str("\n  answer it with `vessel session answer`");
    text
}

/// The complete question content without choosing its caller's command hint.
pub(crate) fn question_form(question: &Question) -> String {
    let mut text = format!("[question {}] {}", question.id, question.message);
    for field in &question.fields {
        let label = field.title.as_deref().unwrap_or(field.name.as_str());
        let required = if field.required { " (required)" } else { "" };
        let _infallible = write!(text, "\n  {}{required}: {label}", field.name);
        match &field.kind {
            FieldKind::Choice { multiple, .. } => {
                let several = if *multiple { ", several" } else { "" };
                let _infallible = write!(text, " (choose{several})");
            }
            FieldKind::Text { .. } => text.push_str(" (text)"),
            FieldKind::Number { .. } => text.push_str(" (number)"),
            FieldKind::Integer { .. } => text.push_str(" (whole number)"),
            FieldKind::Boolean { .. } => text.push_str(" (yes or no)"),
            FieldKind::Unknown => text.push_str(" (a kind this build cannot fill)"),
        }
        if let Some(description) = &field.description {
            let _infallible = write!(text, "\n    {description}");
        }
        if let FieldKind::Choice { options, .. } = &field.kind {
            for option in options {
                let _infallible = write!(text, "\n    {}: {}", option.value, option.title);
                if let Some(description) = &option.description {
                    let _infallible = write!(text, "\n      {description}");
                }
            }
        }
    }
    text
}

/// How a question was settled.
pub(crate) fn question_outcome(outcome: &QuestionOutcome) -> String {
    match outcome {
        QuestionOutcome::Answered { values } => {
            let answers: Vec<String> = values
                .iter()
                .map(|(field, value)| format!("{field}={}", answer(value)))
                .collect();
            format!("answered: {}", answers.join(", "))
        }
        QuestionOutcome::Declined => "declined".to_owned(),
        QuestionOutcome::Cancelled => "withdrawn".to_owned(),
    }
}

fn answer(value: &AnswerValue) -> String {
    match value {
        AnswerValue::Text(text) | AnswerValue::Choice(text) => text.clone(),
        AnswerValue::Number(number) => number.get().to_string(),
        AnswerValue::Integer(number) => number.to_string(),
        AnswerValue::Boolean(yes) => yes.to_string(),
        AnswerValue::Choices(chosen) => chosen.join("|"),
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use vessel_proto::{
        AttachmentId, AttachmentKind, AttachmentRef, QueueEntryId, SessionId, SessionMarks,
    };

    use super::*;

    fn entry(content: Vec<ContentBlock>, by: Creator) -> QueuedPrompt {
        QueuedPrompt {
            provenance: None,
            id: QueueEntryId::random(),
            content,
            queued_at: DateTime::UNIX_EPOCH,
            revision: 2,
            by,

            client_message_id: None,
        }
    }

    #[test]
    fn a_queue_shows_each_entry_on_a_line() {
        let file = ContentBlock::Attachment {
            attachment: AttachmentRef {
                id: AttachmentId::random(),
                name: "notes.txt".parse().unwrap(),
                kind: AttachmentKind::File,
                mime: "text/plain".parse().unwrap(),
                size: 3,
            },
        };
        let agent = SessionId::random();
        let long = "x".repeat(PROMPT_SHOWN + 5);
        let waiting = PromptQueue {
            entries: vec![
                entry(
                    vec![
                        ContentBlock::Text {
                            text: "fix the build\nand the tests".to_owned(),
                        },
                        file,
                    ],
                    Creator::Owner,
                ),
                entry(
                    vec![ContentBlock::Text { text: long }],
                    Creator::Agent {
                        node: vessel_proto::NodeId::random(),
                        session: agent,
                    },
                ),
            ],
            held: Some(QueueHold::Cancelled),
        };
        let text = queue_entries(&waiting);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines[0], "[queue: 2 waiting, held: a turn was cancelled]",
            "{text}"
        );
        let first = &waiting.entries[0].id.to_string();
        assert!(
            lines[1].starts_with(&format!(" 1  {first}  r2  owner")),
            "{text}"
        );
        assert!(lines[1].ends_with("fix the build (+1 file)"), "{text}");
        let short: String = agent.to_string().chars().take(8).collect();
        assert!(lines[2].contains(&format!("agent {short}")), "{text}");
        assert!(lines[2].ends_with('…'), "{text}");
        assert_eq!(
            queue_entries(&PromptQueue::default()),
            "[queue: 0 waiting]\n"
        );
    }

    #[test]
    fn marks_and_deliveries_read_as_words() {
        assert_eq!(marks(&SessionMarks::default()), "");
        let all = SessionMarks {
            pinned: true,
            archived_at: Some(DateTime::UNIX_EPOCH),
            unread: true,
        };
        assert_eq!(marks(&all), "pinned, archived 1970-01-01 00:00, unread");
        let entry = QueueEntryId::random();
        assert_eq!(
            delivered(&Delivered::Queued { entry }),
            format!("queued {entry}")
        );
        assert_eq!(delivered(&Delivered::Steered), "steered");
    }
}
