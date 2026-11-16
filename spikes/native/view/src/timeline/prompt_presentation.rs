//! Display-only voice envelopes. The source remains available to plugins,
//! replay and fork; local find must search the same slice the bubble displays.

use super::is_space;

const OPEN: &str = "<spoken-input>";
const CLOSE: &str = "</spoken-input>";
const INSTRUCTION: &str = "<instruction>This message was spoken by the user.";
const INSTRUCTION_END: &str = "</instruction>";

/// The spoken words of a complete voice envelope, otherwise the original text.
///
/// Paseo's `presentation.ts` anchors the whole wrapper and accepts only its own
/// instruction prefix. This is presentation, never permission or voice-mode
/// admission: quoted examples and unrelated XML keep all their text.
#[must_use]
pub fn presented_prompt(text: &str) -> &str {
    let Some(body) = text.trim_matches(is_space).strip_prefix(OPEN) else {
        return text;
    };
    // Match the first closing tag whose suffix completes the anchored envelope,
    // as the reference's lazy capture does. An inner closing-tag example alone
    // cannot cause the rest of a user's message to disappear.
    body.match_indices(CLOSE)
        .find_map(|(at, _)| {
            let (spoken, tail) = body.split_at(at);
            let suffix = tail.strip_prefix(CLOSE)?.trim_matches(is_space);
            let complete = suffix.is_empty()
                || suffix
                    .strip_prefix(INSTRUCTION)
                    .is_some_and(|instruction| instruction.ends_with(INSTRUCTION_END));
            complete.then(|| spoken.trim_matches(is_space))
        })
        .unwrap_or(text)
}

#[cfg(test)]
mod tests;
