//! Matching words without case, and the window of a message a hit shows.
//!
//! Every candidate a walk reads is checked here against every word, and
//! every word is marked in a snippet here, with one folding ([`fold`]): the
//! one `SQLite`'s trigram tokenizer applies (case, nothing else), one
//! character to one character, so a position in the folded text is the
//! same position in the text shown. The index only narrows what is read.

use vessel_proto::search::{Snippet, SnippetPart};

/// Characters a snippet shows before its first match.
const LEAD: usize = 24;

/// One character without case: its lower case when that is a single
/// character (nearly every one), else itself (`İ` lowers to two).
pub(super) fn fold(c: char) -> char {
    let mut lower = c.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(one), None) => one,
        _ => c,
    }
}

/// `text` without case, for [`contains_all`].
pub(super) fn folded(text: &str) -> String {
    text.chars().map(fold).collect()
}

/// Whether `text` (already [`folded`]) holds every word (each folded).
pub(super) fn contains_all(text: &str, words: &[String]) -> bool {
    words.iter().all(|word| text.contains(word.as_str()))
}

/// The window of `text` around the first of `words` (each folded), white
/// space runs as one space, every word inside it marked.
pub(super) fn snippet(text: &str, words: &[String]) -> Snippet {
    let shown: Vec<char> = normalised(text);
    let folded: String = shown.iter().copied().map(fold).collect();
    // Byte offset of each folded character, to turn a match's byte
    // position into a character position.
    let starts: Vec<usize> = folded.char_indices().map(|(at, _)| at).collect();
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for word in words.iter().filter(|word| !word.is_empty()) {
        let length = word.chars().count();
        for (byte, _) in folded.match_indices(word.as_str()) {
            if let Ok(at) = starts.binary_search(&byte) {
                ranges.push((at, at.saturating_add(length)));
            }
        }
    }
    ranges.sort_unstable();
    let ranges = merged(ranges);
    let total = shown.len();
    let (start, end) = window(total, ranges.first().map_or(0, |range| range.0));
    Snippet {
        parts: parts(&shown, &ranges, start, end),
        cut_before: start > 0,
        cut_after: end < total,
    }
}

/// `text` as characters, each run of white space one space, trimmed.
fn normalised(text: &str) -> Vec<char> {
    let mut shown = Vec::with_capacity(text.len());
    for word in text.split_whitespace() {
        if !shown.is_empty() {
            shown.push(' ');
        }
        shown.extend(word.chars());
    }
    shown
}

/// Overlapping or touching ranges as one.
fn merged(sorted: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    let mut merged: Vec<(usize, usize)> = Vec::with_capacity(sorted.len());
    for (start, end) in sorted {
        match merged.last_mut() {
            Some(last) if start <= last.1 => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }
    merged
}

/// The shown window of a text of `total` characters whose first match is at
/// `first`: up to [`LEAD`] characters before it, [`Snippet::MAX_CHARS`] in
/// all, moved back when the text ends sooner.
fn window(total: usize, first: usize) -> (usize, usize) {
    if total <= Snippet::MAX_CHARS {
        return (0, total);
    }
    let start = first.saturating_sub(LEAD);
    let end = start.saturating_add(Snippet::MAX_CHARS).min(total);
    (end.saturating_sub(Snippet::MAX_CHARS), end)
}

/// The window's text as parts, matched or not.
fn parts(shown: &[char], ranges: &[(usize, usize)], start: usize, end: usize) -> Vec<SnippetPart> {
    let mut parts = Vec::new();
    let mut at = start;
    let text = |from: usize, to: usize| -> String {
        shown
            .get(from..to)
            .map_or_else(String::new, |chars| chars.iter().collect())
    };
    for &(from, to) in ranges {
        let (from, to) = (from.max(start), to.min(end));
        if from >= to {
            continue;
        }
        if from > at {
            parts.push(SnippetPart {
                text: text(at, from),
                matched: false,
            });
        }
        parts.push(SnippetPart {
            text: text(from, to),
            matched: true,
        });
        at = to;
    }
    if at < end {
        parts.push(SnippetPart {
            text: text(at, end),
            matched: false,
        });
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| folded(word)).collect()
    }

    fn marked(snippet: &Snippet) -> String {
        snippet
            .parts
            .iter()
            .map(|part| {
                if part.matched {
                    format!("[{}]", part.text)
                } else {
                    part.text.clone()
                }
            })
            .collect()
    }

    #[test]
    fn a_short_message_is_shown_whole_with_its_matches_marked() {
        let snippet = snippet(
            "The  Parser\nfailed; the parser   again",
            &words(&["parser"]),
        );
        assert_eq!(marked(&snippet), "The [Parser] failed; the [parser] again");
        assert!(!snippet.cut_before && !snippet.cut_after);
    }

    #[test]
    fn hangul_is_found_inside_a_word_with_its_particle() {
        let snippet = snippet("세션을 검색합니다", &words(&["세션", "검색"]));
        assert_eq!(marked(&snippet), "[세션]을 [검색]합니다");
        assert!(contains_all(
            &folded("세션을 검색합니다"),
            &words(&["세션", "검색"])
        ));
        assert!(!contains_all(&folded("세션을"), &words(&["세션", "검색"])));
    }

    #[test]
    fn a_long_message_shows_a_window_that_starts_before_the_first_match() {
        let text = format!("{} needle {}", "a".repeat(500), "b".repeat(500));
        let snippet = snippet(&text, &words(&["NEEDLE"]));
        let shown: String = snippet
            .parts
            .iter()
            .map(|part| part.text.as_str())
            .collect();
        assert_eq!(shown.chars().count(), Snippet::MAX_CHARS);
        assert!(snippet.cut_before && snippet.cut_after);
        assert!(shown.starts_with(&"a".repeat(LEAD - 1)), "{shown}");
        assert_eq!(snippet.parts.iter().filter(|part| part.matched).count(), 1);
    }

    #[test]
    fn a_match_near_the_end_moves_the_window_back() {
        let text = format!("{}needle", "a".repeat(1000));
        let snippet = snippet(&text, &words(&["needle"]));
        assert!(snippet.cut_before && !snippet.cut_after);
        assert_eq!(snippet.parts.last().map(|part| part.matched), Some(true));
    }

    #[test]
    fn overlapping_words_are_one_mark_and_no_match_shows_the_start() {
        assert_eq!(
            marked(&snippet("abcdef", &words(&["abc", "cde"]))),
            "[abcde]f"
        );
        let snippet = snippet(&"x".repeat(300), &words(&["café"]));
        assert!(!snippet.cut_before && snippet.cut_after);
    }
}
