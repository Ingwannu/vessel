//! Find in a session: what matches a query in the timeline's entries,
//! never in what is drawn (the page's `data/find.ts`).
//!
//! [Decision Log]
//! - Purpose: all three references find text in a session (Mod+F); the
//!   timeline is windowed, so text not drawn must still be found and its
//!   row reachable.
//! - Constraint: a session can hold thousands of entries; a search on
//!   every keystroke must not hold a frame.
//! - Choice (the page's, kept): prompts, steers, questions and the agent's
//!   messages are searched, not tool output. Matching folds case, accents
//!   and runs of white space, over the text as it reads: Markdown's marks
//!   are removed first (`readable_text`). Each entry's folded text is
//!   kept by entry index and revision (`FindCache`), so a second search
//!   folds only what changed; a long search runs in slices (`Finder`),
//!   one slice per frame, and a newer query replaces it.
//! - Differences from the page, each about an edge the page calls
//!   approximate: a Markdown line's marks are read within the line (the
//!   page's regular expressions can reach across a line break after a
//!   heading mark); "a letter or a number" beside an underscore is Rust's
//!   `char::is_alphanumeric`, where the page's is `\p{L}\p{N}`; offsets
//!   are bytes of UTF-8, where the page's are UTF-16 units (the same
//!   text either way).

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;
use vessel_proto::EventSeq;

use super::entry::{Entry, EntryKind};

mod cache;
pub use cache::FindCache;

/// One occurrence: the entry it is in and its index there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    /// The entry's index.
    pub entry: usize,
    /// Which occurrence within the entry, from 0.
    pub nth: u32,
}

/// The longest query searched, in characters (agent-orchestrator's
/// limit).
pub const MAX_QUERY: usize = 512;

/// Entries searched per slice.
pub const SLICE: usize = 200;

pub use super::searchable::searchable;

/// White space as the page's `\s` reads it (ECMAScript's set), so a fold
/// here and one in the page agree on what a run of space is.
#[must_use]
pub const fn is_space(character: char) -> bool {
    matches!(
        character,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// `text` folded for matching: decomposed, accents dropped, lower case,
/// white space runs as one space.
#[must_use]
pub fn fold(text: &str) -> String {
    let mut folded = String::with_capacity(text.len());
    for character in text.chars() {
        push_folded(&mut folded, character);
    }
    folded
}

/// One character folded onto `into`: a space joins a space before it.
fn push_folded(into: &mut String, character: char) {
    if is_space(character) {
        if !into.ends_with(' ') {
            into.push(' ');
        }
        return;
    }
    // ASCII needs no decomposition: most of a log is ASCII, and this
    // keeps a cold search over thousands of messages short.
    if character.is_ascii() {
        into.push(character.to_ascii_lowercase());
        return;
    }
    for part in std::iter::once(character).nfd() {
        if !is_combining_mark(part) {
            into.extend(part.to_lowercase());
        }
    }
}

/// The query as it is matched: its first `MAX_QUERY` characters, folded
/// and trimmed; empty searches nothing.
#[must_use]
pub fn query_of(text: &str) -> String {
    let cut: String = text.chars().take(MAX_QUERY).collect();
    fold(&cut).trim_matches(' ').to_owned()
}

/// How many times `needle` occurs in `haystack`, not overlapping.
#[must_use]
pub fn occurrences(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    haystack.matches(needle).count()
}

/// Markdown's marks removed: fences, links as their words, heading,
/// quote and list marks, emphasis and code ticks. Approximate on purpose:
/// it decides what a reader would call the text, not how it is drawn. An
/// underscore inside a word (`module_300`, `snake_case`) is the word's,
/// as `CommonMark` reads it; only underscores at a word's edge mark
/// emphasis.
#[must_use]
pub fn readable_text(markdown: &str) -> String {
    let lines: Vec<&str> = markdown.split('\n').collect();
    let unfenced: Vec<&str> = lines
        .iter()
        .map(|line| if is_fence(line) { "" } else { line })
        .collect();
    let linked = links_as_words(&unfenced.join("\n"));
    let unmarked: Vec<&str> = linked.split('\n').map(without_line_mark).collect();
    let plain = without_emphasis(&unmarked.join("\n"));
    without_edge_underscores(&plain)
}

/// Up to three spaces at the start of `line`, skipped.
fn after_indent(line: &str) -> &str {
    let mut rest = line;
    for _ in 0..3 {
        match rest.chars().next() {
            Some(character) if is_space(character) => {
                rest = rest.get(character.len_utf8()..).unwrap_or_default();
            }
            Some(_) | None => break,
        }
    }
    rest
}

fn is_fence(line: &str) -> bool {
    let rest = after_indent(line);
    rest.starts_with("```") || rest.starts_with("~~~")
}

/// At least one space at the start of `text`: what follows them, or
/// `None` when there is none.
fn after_spaces(text: &str) -> Option<&str> {
    let rest = text.trim_start_matches(is_space);
    (rest.len() < text.len()).then_some(rest)
}

/// `line` without its heading, quote or list mark (and the up to three
/// spaces before it); as it was when it has none.
fn without_line_mark(line: &str) -> &str {
    let rest = after_indent(line);
    let hashes = rest
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if (1..=6).contains(&hashes)
        && let Some(after) = rest.get(hashes..).and_then(after_spaces)
    {
        return after;
    }
    if let Some(after) = rest.strip_prefix('>') {
        return after
            .strip_prefix(|character: char| is_space(character))
            .unwrap_or(after);
    }
    if let Some(after) = rest.strip_prefix(['-', '*', '+']).and_then(after_spaces) {
        return after;
    }
    let digits = rest.chars().take_while(char::is_ascii_digit).count();
    if digits > 0
        && let Some(after) = rest
            .get(digits..)
            .and_then(|tail| tail.strip_prefix(['.', ')']))
            .and_then(after_spaces)
    {
        return after;
    }
    line
}

/// `[words](target)` and `![words](target)` as their words.
fn links_as_words(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(character) = rest.chars().next() {
        if let Some((words, after)) = link_at(rest) {
            out.push_str(words);
            rest = after;
            continue;
        }
        out.push(character);
        rest = rest.get(character.len_utf8()..).unwrap_or_default();
    }
    out
}

/// A link at the start of `text`: its words and what follows it.
fn link_at(text: &str) -> Option<(&str, &str)> {
    let open = text.strip_prefix('!').unwrap_or(text);
    let inner = open.strip_prefix('[')?;
    let close = inner.find(']')?;
    let words = inner.get(..close)?;
    let target = inner.get(close.checked_add(1)?..)?.strip_prefix('(')?;
    let end = target.find(')')?;
    Some((words, target.get(end.checked_add(1)?..)?))
}

/// Emphasis marks and code ticks removed: every `*` and backtick, and
/// `~~` (a lone `~` is text).
fn without_emphasis(text: &str) -> String {
    text.replace("~~", "").replace(['*', '`'], "")
}

/// Underscores that mark emphasis removed: every run of them except a
/// single one between two letters or numbers.
fn without_edge_underscores(text: &str) -> String {
    let characters: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0_usize;
    while let Some(character) = characters.get(index).copied() {
        if character != '_' {
            out.push(character);
            index = index.saturating_add(1);
            continue;
        }
        let start = index;
        while characters.get(index) == Some(&'_') {
            index = index.saturating_add(1);
        }
        let word = |at: Option<usize>| {
            at.and_then(|at| characters.get(at))
                .is_some_and(|character| character.is_alphanumeric())
        };
        let inside =
            index.saturating_sub(start) == 1 && word(start.checked_sub(1)) && word(Some(index));
        if inside {
            out.push('_');
        }
    }
    out
}

/// Where `query` occurs in `text` as find matches it (folded as `fold`
/// folds, so the highlight and the count agree: "cafe" marks "café"), as
/// byte ranges of `text` itself, not overlapping, in order.
#[must_use]
pub fn spans_of(text: &str, query: &str) -> Vec<std::ops::Range<usize>> {
    let needle = query_of(query);
    if needle.is_empty() {
        return Vec::new();
    }
    // The folded text; for each of its characters, where it starts in
    // the folded text and which bytes of `text` it came from.
    let mut folded = String::with_capacity(text.len());
    let mut starts: Vec<usize> = Vec::new();
    let mut origins: Vec<std::ops::Range<usize>> = Vec::new();
    for (at, character) in text.char_indices() {
        let next = at.saturating_add(character.len_utf8());
        let mut one = String::new();
        push_folded(&mut one, character);
        if one.is_empty() {
            // A mark written apart belongs to the letter before it.
            if let Some(last) = origins.last_mut() {
                last.end = next;
            }
            continue;
        }
        if one == " " && folded.ends_with(' ') {
            continue;
        }
        for part in one.chars() {
            starts.push(folded.len());
            origins.push(at..next);
            folded.push(part);
        }
    }
    let mut spans = Vec::new();
    for (found, _) in folded.match_indices(needle.as_str()) {
        let end = found.saturating_add(needle.len());
        let first = starts.partition_point(|start| *start < found);
        let last = starts.partition_point(|start| *start < end).checked_sub(1);
        if let (Some(from), Some(to)) =
            (origins.get(first), last.and_then(|last| origins.get(last)))
        {
            spans.push(from.start..to.end);
        }
    }
    spans
}

/// Every match of `query` in `entries`, in order, at once.
#[must_use]
pub fn find_all(cache: &mut FindCache, entries: &[Entry], query: &str) -> Vec<Match> {
    let needle = query_of(query);
    let mut matches = Vec::new();
    if needle.is_empty() {
        return matches;
    }
    for (index, entry) in entries.iter().enumerate() {
        cache.collect(index, entry, &needle, &mut matches);
    }
    matches
}

/// A search over a long log, `SLICE` entries per step: the screen runs one
/// step a frame, so typing never waits on it, and drops the finder when
/// a newer query starts.
#[derive(Debug)]
pub struct Finder {
    needle: String,
    at: usize,
    matches: Vec<Match>,
}

impl Finder {
    /// A search for `query` from the first entry.
    #[must_use]
    pub fn new(query: &str) -> Self {
        Self {
            needle: query_of(query),
            at: 0,
            matches: Vec::new(),
        }
    }

    /// One slice searched; every match once the search reached the end
    /// of `entries`, `None` while it has not.
    pub fn step(&mut self, cache: &mut FindCache, entries: &[Entry]) -> Option<Vec<Match>> {
        if self.needle.is_empty() {
            return Some(Vec::new());
        }
        let end = self.at.saturating_add(SLICE).min(entries.len());
        for index in self.at..end {
            if let Some(entry) = entries.get(index) {
                cache.collect(index, entry, &self.needle, &mut self.matches);
            }
        }
        self.at = end;
        (end >= entries.len()).then(|| std::mem::take(&mut self.matches))
    }
}

/// Where a step through the matches lands: the index, and whether it
/// wrapped (which the bar says aloud).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stepped {
    /// The match now current.
    pub index: usize,
    /// Whether it went round an end.
    pub wrapped: bool,
}

/// The match after (`forward`) or before `current`, among `count`,
/// wrapping.
#[must_use]
pub fn step(count: usize, current: usize, forward: bool) -> Stepped {
    if count == 0 {
        return Stepped {
            index: 0,
            wrapped: false,
        };
    }
    let last = count.saturating_sub(1);
    match (forward, current) {
        (true, at) if at >= last => Stepped {
            index: 0,
            wrapped: true,
        },
        (true, at) => Stepped {
            index: at.saturating_add(1),
            wrapped: false,
        },
        (false, 0) => Stepped {
            index: last,
            wrapped: true,
        },
        (false, at) => Stepped {
            index: at.saturating_sub(1),
            wrapped: false,
        },
    }
}

/// Where find starts for the message at record `at` (a search hit opened
/// from the palette): the first match in its entry or after it, the first
/// match when none is; `None` while the session is not read that far
/// (`next`, the record after the last read).
#[must_use]
pub fn start_at(
    entries: &[Entry],
    next: EventSeq,
    matches: &[Match],
    at: EventSeq,
) -> Option<usize> {
    if next <= at {
        return None;
    }
    let entry = super::entry_at_source(entries, at)
        .or_else(|| entries.iter().rposition(|entry| entry.seq <= at))
        .unwrap_or(0);
    Some(
        matches
            .iter()
            .position(|found| found.entry >= entry)
            .unwrap_or(0),
    )
}
