//! API-specific finality and content checks, using the existing wire grammar.

use super::{Completion, CompletionError as Error, finish, json, only};
use crate::{
    ir, upstream,
    wire::{TextOr, anthropic as a, chat as c, responses as r},
};

pub(super) fn anthropic(text: &str) -> Result<Completion, Error> {
    let reply: a::Message = json(text)?;
    let usage = upstream::usage::anthropic(&reply.usage);
    let stop = match reply.stop_reason {
        None | Some(a::StopReason::Unknown(_)) => None,
        Some(reason) => Some(upstream::anthropic::stop_reason(&reason)),
    };
    let text = if reply.role != a::Role::Assistant || reply.stop_sequence.is_some() {
        Err(Error::Unsupported)
    } else {
        reply
            .content
            .into_iter()
            .try_fold(String::new(), |mut text, block| {
                match block {
                    a::ContentBlock::Text(block) => text.push_str(&block.text),
                    a::ContentBlock::Thinking(_) | a::ContentBlock::RedactedThinking(_) => {}
                    a::ContentBlock::ToolUse(_) | a::ContentBlock::ToolResult(_) => {
                        return Err(Error::Tool);
                    }
                    a::ContentBlock::Image(_)
                    | a::ContentBlock::Document(_)
                    | a::ContentBlock::Unknown { .. } => return Err(Error::Unsupported),
                }
                Ok(text)
            })
    };
    Ok(finish(usage, stop, text))
}

pub(super) fn chat(text: &str) -> Result<Completion, Error> {
    let reply: c::CompletionResponse = json(text)?;
    let usage = reply
        .usage
        .as_ref()
        .map(upstream::usage::chat)
        .unwrap_or_default();
    let choice = match only(reply.choices) {
        Ok(choice) => choice,
        Err(error) => return Ok(finish(usage, None, Err(error))),
    };
    let stop = match &choice.finish_reason {
        Some(c::FinishReason::Stop) => Some(ir::StopReason::EndTurn),
        Some(c::FinishReason::Length) => Some(ir::StopReason::MaxTokens),
        Some(c::FinishReason::ContentFilter) => Some(ir::StopReason::Refusal),
        Some(c::FinishReason::ToolCalls | c::FinishReason::FunctionCall) => {
            Some(ir::StopReason::ToolUse)
        }
        Some(c::FinishReason::Error | c::FinishReason::Unknown(_)) | None => None,
    };
    let message = choice.message;
    let text = if matches!(choice.finish_reason, Some(c::FinishReason::Error)) {
        Err(Error::Failed)
    } else if message.refusal.is_some() {
        Err(Error::Refusal)
    } else if message.function_call.is_some() || !message.body.tool_calls.is_empty() {
        Err(Error::Tool)
    } else {
        match message.body.content {
            Some(TextOr::Text(text)) => Ok(text),
            Some(TextOr::Parts(parts)) => {
                parts.into_iter().try_fold(String::new(), |mut text, part| {
                    match part {
                        c::ContentPart::Text(part) => text.push_str(&part.text),
                        c::ContentPart::ImageUrl(_) | c::ContentPart::Unknown { .. } => {
                            return Err(Error::Unsupported);
                        }
                    }
                    Ok(text)
                })
            }
            None => Err(Error::Empty),
        }
    };
    Ok(finish(usage, stop, text))
}

pub(super) fn responses(text: &str) -> Result<Completion, Error> {
    let reply: r::Response = json(text)?;
    let usage = reply
        .usage
        .as_ref()
        .map(upstream::usage::responses)
        .unwrap_or_default();
    let failed = matches!(reply.status, r::ResponseStatus::Failed);
    let stop = response_stop(&reply.status, reply.incomplete_details);
    let text = if failed {
        Err(Error::Failed)
    } else {
        let messages = reply
            .output
            .into_iter()
            .filter_map(|item| match item {
                r::Item::Message(message) => Some(Ok(message)),
                r::Item::Reasoning(_) => None,
                r::Item::FunctionCall(_)
                | r::Item::FunctionCallOutput(_)
                | r::Item::CustomToolCall(_)
                | r::Item::CustomToolCallOutput(_) => Some(Err(Error::Tool)),
                r::Item::Unknown { .. } => Some(Err(Error::Unsupported)),
            })
            .collect::<Result<Vec<_>, _>>();
        messages.and_then(only).and_then(response_text)
    };
    Ok(finish(usage, stop, text))
}

fn response_stop(
    status: &r::ResponseStatus,
    details: Option<r::IncompleteDetails>,
) -> Option<ir::StopReason> {
    match status {
        r::ResponseStatus::Completed if details.is_none() => Some(ir::StopReason::EndTurn),
        r::ResponseStatus::Completed | r::ResponseStatus::Incomplete => {
            match details.and_then(|details| details.reason) {
                Some(r::IncompleteReason::MaxOutputTokens) => Some(ir::StopReason::MaxTokens),
                Some(r::IncompleteReason::ContentFilter) => Some(ir::StopReason::Refusal),
                Some(r::IncompleteReason::Unknown(_)) | None => None,
            }
        }
        r::ResponseStatus::Queued
        | r::ResponseStatus::InProgress
        | r::ResponseStatus::Failed
        | r::ResponseStatus::Cancelled
        | r::ResponseStatus::Unknown(_) => None,
    }
}

fn response_text(message: r::MessageItem) -> Result<String, Error> {
    if message.role != r::Role::Assistant
        || message
            .status
            .as_ref()
            .is_some_and(|status| *status != r::ItemStatus::Completed)
    {
        return Err(Error::Unsupported);
    }
    match message.content {
        TextOr::Text(text) => Ok(text),
        TextOr::Parts(parts) => parts.into_iter().try_fold(String::new(), |mut text, part| {
            match part {
                r::ContentPart::OutputText(part) => text.push_str(&part.text),
                r::ContentPart::Refusal(_) => return Err(Error::Refusal),
                r::ContentPart::InputText(_)
                | r::ContentPart::InputImage(_)
                | r::ContentPart::InputFile(_)
                | r::ContentPart::Unknown { .. } => return Err(Error::Unsupported),
            }
            Ok(text)
        }),
    }
}
