//! Bounded TTS sentence/clause/word splitting without cutting a Unicode scalar.
//!
//! Paseo counts UTF-16 code units. Rust retains that limit while moving a hard
//! cut before a surrogate pair; emitting an unpaired surrogate is impossible
//! in validated UTF-8. This is the only intentional split-boundary difference.

use vessel_proto::voice::{SpeechText, VoiceDecodeError};
use vessel_proto::workspace::trim_project_text;

pub(super) const MAX_SEGMENT_UNITS: usize = 260;
const MIN_WORD_BREAK_UNITS: usize = 130;

pub(super) fn split(text: &SpeechText) -> Result<Vec<SpeechText>, VoiceDecodeError> {
    let normalized = normalize(text.as_str());
    let mut result = Vec::new();
    for sentence in boundaries(&normalized, &['.', '!', '?']) {
        for fragment in fragment(sentence) {
            result.push(SpeechText::try_from(fragment)?);
        }
    }
    Ok(result)
}

fn normalize(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut pending_space = false;
    for character in text.chars() {
        let mut storage = [0; 4];
        let whitespace = trim_project_text(character.encode_utf8(&mut storage)).is_empty();
        if whitespace {
            pending_space = !normalized.is_empty();
        } else {
            if pending_space {
                normalized.push(' ');
                pending_space = false;
            }
            normalized.push(character);
        }
    }
    normalized
}

fn units(text: &str) -> usize {
    text.encode_utf16().count()
}

fn boundaries<'a>(text: &'a str, punctuation: &[char]) -> Vec<&'a str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut previous = None;
    for (index, character) in text.char_indices() {
        if character == ' ' && previous.is_some_and(|p| punctuation.contains(&p)) {
            if let Some(part) = text.get(start..index) {
                parts.push(part);
            }
            start = index.saturating_add(character.len_utf8());
        }
        previous = Some(character);
    }
    if let Some(last) = text.get(start..) {
        if !last.is_empty() {
            parts.push(last);
        }
    }
    parts
}

fn fragment(text: &str) -> Vec<String> {
    if units(text) <= MAX_SEGMENT_UNITS {
        return vec![text.to_owned()];
    }
    let mut parts = Vec::new();
    let mut current = String::new();
    for clause in boundaries(text, &[',', ';', ':']) {
        if units(clause) > MAX_SEGMENT_UNITS {
            push(&mut parts, &mut current);
            hard_split(clause, &mut parts);
        } else {
            let required = units(&current)
                .saturating_add(usize::from(!current.is_empty()))
                .saturating_add(units(clause));
            if required > MAX_SEGMENT_UNITS {
                push(&mut parts, &mut current);
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(clause);
        }
    }
    push(&mut parts, &mut current);
    parts
}

fn push(parts: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        parts.push(std::mem::take(current));
    }
}

fn hard_split(mut remaining: &str, parts: &mut Vec<String>) {
    while units(remaining) > MAX_SEGMENT_UNITS {
        let cut = cut_at(remaining);
        let Some((part, rest)) = remaining.split_at_checked(cut) else {
            return;
        };
        parts.push(trim_project_text(part).to_owned());
        remaining = trim_project_text(rest);
    }
    if !remaining.is_empty() {
        parts.push(remaining.to_owned());
    }
}

fn cut_at(text: &str) -> usize {
    let mut count = 0_usize;
    let mut hard = 0;
    let mut word = None;
    for (index, character) in text.char_indices() {
        if count > MAX_SEGMENT_UNITS {
            break;
        }
        if character == ' ' && count >= MIN_WORD_BREAK_UNITS {
            word = Some(index);
        }
        let next = count.saturating_add(character.len_utf16());
        if next > MAX_SEGMENT_UNITS {
            break;
        }
        count = next;
        hard = index.saturating_add(character.len_utf8());
    }
    word.unwrap_or(hard)
}

#[cfg(test)]
mod tests;
