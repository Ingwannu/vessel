//! One accounting convention for streamed and complete API replies. Keeping
//! conversion here prevents metadata calls from counting cache tokens twice.

use crate::ir;

pub(crate) fn update_anthropic(current: &mut ir::Usage, usage: &crate::wire::anthropic::Usage) {
    if let Some(input) = usage.input_tokens {
        current.input = input;
    }
    if let Some(read) = usage.cache_read_input_tokens {
        current.cache_read = read;
    }
    if let Some(write) = usage.cache_creation_input_tokens {
        current.cache_write = write;
    }
    if let Some(output) = usage.output_tokens {
        current.output = output;
    }
}

pub(crate) fn anthropic(usage: &crate::wire::anthropic::Usage) -> ir::Usage {
    let mut current = ir::Usage::default();
    update_anthropic(&mut current, usage);
    current
}

pub(crate) fn chat(usage: &crate::wire::chat::Usage) -> ir::Usage {
    let cached = usage
        .prompt_tokens_details
        .and_then(|details| details.cached_tokens)
        .unwrap_or(0);
    ir::Usage {
        input: usage.prompt_tokens.saturating_sub(cached),
        cache_read: cached,
        cache_write: 0,
        output: usage.completion_tokens,
        reasoning: usage
            .completion_tokens_details
            .and_then(|details| details.reasoning_tokens)
            .unwrap_or(0),
    }
}

pub(crate) fn responses(usage: &crate::wire::responses::ResponseUsage) -> ir::Usage {
    let cached = usage.input_tokens_details.cached_tokens;
    ir::Usage {
        input: usage.input_tokens.saturating_sub(cached),
        cache_read: cached,
        cache_write: 0,
        output: usage.output_tokens,
        reasoning: usage.output_tokens_details.reasoning_tokens,
    }
}
