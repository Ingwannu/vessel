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
