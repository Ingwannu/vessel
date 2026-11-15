//! Markdown file preview framing follows Paseo's markdown-preview/document.ts.
//! The bounded parser returns display rows and body, never local resource I/O.

mod metadata;
use std::sync::Arc;

/// Native plain-text budget also bounds a preview's retained source.
pub const MAX_MARKDOWN_BYTES: usize = 10 * 1024 * 1024;
/// Front matter is metadata, not a second unbounded document.
pub const MAX_FRONT_MATTER_BYTES: usize = 64 * 1024;
/// Paseo's alias count, plus explicit depth/event work ceilings.
pub const MAX_FRONT_MATTER_ALIASES: usize = 100;
/// Bound recursive metadata and serialization stack use.
pub const MAX_FRONT_MATTER_DEPTH: usize = 16;
/// Bound retained/replayed alias work before allocating a metadata table.
pub const MAX_FRONT_MATTER_EVENTS: usize = 4096;

/// A complete bounded source, shared between worker input and fallback output.
#[derive(Debug, Clone)]
pub struct MarkdownSource(Arc<str>);

/// Refuse oversized preview input before parsing or cloning document bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Markdown previews accept at most {MAX_MARKDOWN_BYTES} bytes")]
pub struct MarkdownTooLarge;

impl TryFrom<Arc<str>> for MarkdownSource {
    type Error = MarkdownTooLarge;
    fn try_from(source: Arc<str>) -> Result<Self, Self::Error> {
        if source.len() > MAX_MARKDOWN_BYTES {
            Err(MarkdownTooLarge)
        } else {
            Ok(Self(source))
        }
    }
}

/// One metadata table row, as plain display text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontMatterRow {
    /// Exact YAML scalar key, preserving author order.
    pub key: String,
    /// Scalar/list display or bounded YAML for nested values.
    pub value: String,
}

/// Preview data; a parse error keeps the entire source as ordinary Markdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownDocument {
    /// Empty when no valid metadata mapping exists.
    pub front_matter: Vec<FrontMatterRow>,
    /// Complete source body, never a partial page or an error message.
    pub body: Arc<str>,
}

/// Parse bounded metadata; callers schedule this off the UI thread when source
/// changes, rather than rebuilding it on cursor or animation frames.
#[must_use]
pub fn parse(source: &MarkdownSource) -> MarkdownDocument {
    let parsed = frame(&source.0)
        .and_then(|(header, body)| Some((metadata::rows(header)?, Arc::from(body))));
    match parsed {
        Some((front_matter, body)) => MarkdownDocument { front_matter, body },
        None => MarkdownDocument {
            front_matter: Vec::new(),
            body: Arc::clone(&source.0),
        },
    }
}

fn marker(line: &str) -> bool {
    let line = line.strip_suffix('\n').unwrap_or(line);
    let line = line.strip_suffix('\r').unwrap_or(line);
    line.strip_prefix("---").is_some_and(|tail| {
        tail.chars()
            .all(|character| matches!(character, ' ' | '\t'))
    })
}

fn frame(source: &str) -> Option<(&str, &str)> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let (opening, rest) = source.split_once('\n')?;
    if !marker(opening) {
        return None;
    }
    let mut consumed = 0_usize;
    for line in rest.split_inclusive('\n') {
        if marker(line) {
            let header = rest.get(..consumed)?;
            if header.len() > MAX_FRONT_MATTER_BYTES {
                return None;
            }
            let end = consumed.checked_add(line.len())?;
            return Some((header, rest.get(end..)?));
        }
        consumed = consumed.checked_add(line.len())?;
        if consumed > MAX_FRONT_MATTER_BYTES {
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests;
