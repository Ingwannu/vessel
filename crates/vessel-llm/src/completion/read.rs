//! API-specific finality and content checks, using the existing wire grammar.

use super::{Completion, CompletionError as Error, finish, json};
use crate::{upstream, wire::anthropic as a};

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
