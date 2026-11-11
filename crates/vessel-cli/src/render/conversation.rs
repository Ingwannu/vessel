//! Protocol 10's conversation as text (Track A): the queue, the agent's
//! commands and context, and its questions, one line or block each, for
//! `vessel attach` and `vessel log`.

use std::fmt::Write as _;

use vessel_proto::conversation::{AnswerValue, FieldKind};
use vessel_proto::{AgentCommand, ContextUsage, PromptQueue, Question, QuestionOutcome, QueueHold};

/// How many commands a line names before it counts the rest.
const COMMANDS_SHOWN: usize = 12;

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
    let mut text = format!("[question {}] {}", question.id, question.message);
    for field in &question.fields {
        let label = field.title.as_deref().unwrap_or(field.name.as_str());
        let required = if field.required { " (required)" } else { "" };
        let _infallible = write!(text, "\n  {}{required}: {label}", field.name);
        match &field.kind {
            FieldKind::Choice {
                options, multiple, ..
            } => {
                let several = if *multiple { ", several" } else { "" };
                let _infallible = write!(text, " (choose{several})");
                for option in options {
                    let _infallible = write!(text, "\n    {}: {}", option.value, option.title);
                }
            }
            FieldKind::Text { .. } => text.push_str(" (text)"),
            FieldKind::Number { .. } => text.push_str(" (number)"),
            FieldKind::Integer { .. } => text.push_str(" (whole number)"),
            FieldKind::Boolean { .. } => text.push_str(" (yes or no)"),
            FieldKind::Unknown => text.push_str(" (a kind this build cannot fill)"),
        }
    }
    text.push_str("\n  answer it with `vessel session answer`");
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
