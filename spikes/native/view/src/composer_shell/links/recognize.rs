//! Paseo source.ts:12-55 token boundaries, followed by its file resolver classification.
//! Whole inline code is classified once; prose scans only bounded path-shaped words.

use std::ops::Range;

use super::{EXTENSIONS, Origin, Workspace, classify};

/// Code spans may contain brackets/spaces; prose uses conservative path-word boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Ordinary paragraph text.
    Text,
    /// One complete inline-code span.
    Code,
}

/// A rendered UTF-8 byte range; source mapping remains the existing engine's job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// The unchanged token bytes, suitable as an inferred destination.
    pub range: Range<usize>,
}

/// Recognition grants no path authority; the click handler still resolves its workspace.
#[must_use]
pub fn tokens(text: &str, kind: Kind) -> Vec<Token> {
    if kind == Kind::Code {
        return classify(text, Origin::Inferred, Workspace::Known)
            .is_some()
            .then_some(Token {
                range: 0..text.len(),
            })
            .into_iter()
            .collect();
    }
    let mut result = Vec::new();
    let mut start = None;
    for (index, ch) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        if path_character(ch) {
            if start.is_none() {
                start = Some(index);
            }
        } else if let Some(begin) = start.take() {
            let Some(candidate) = text.get(begin..index) else {
                continue;
            };
            let candidate = candidate.trim_end_matches('.');
            let end = begin.saturating_add(candidate.len());
            if super::http_url(candidate).is_some() || prose_path(candidate) {
                result.push(Token { range: begin..end });
            }
        }
    }
    result
}

fn path_character(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
        || ch >= '\u{80}'
        || matches!(
            ch,
            '_' | '@' | '.' | '%' | '+' | '-' | '/' | '\\' | ':' | '~'
        )
}

// The reference's FILE_PATTERN recognizes syntactic locations first. Invalid
// line values still become links; the parser refuses them on activation.
fn prose_path(value: &str) -> bool {
    let mut path = value;
    let mut located = false;
    for (index, _) in value.match_indices(':') {
        if value
            .get(index.saturating_add(1)..)
            .is_some_and(line_syntax)
        {
            path = value.get(..index).unwrap_or_default();
            located = true;
            break;
        }
    }
    let normalized = path.replace('\\', "/");
    let rest = if normalized.as_bytes().get(1) == Some(&b':')
        && normalized
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic)
    {
        normalized
            .get(2..)
            .and_then(|value| value.strip_prefix('/'))
    } else {
        ["../", "./", "~/", "/"]
            .into_iter()
            .find_map(|prefix| normalized.strip_prefix(prefix))
    };
    if let Some(rest) = rest {
        let segments: Vec<_> = rest.split('/').collect();
        return segments.len() >= 2
            && rest.bytes().any(|byte| byte.is_ascii_alphabetic())
            && segments.iter().all(|segment| path_segment(segment));
    }
    let segments: Vec<_> = normalized.split('/').collect();
    if segments.len() == 1 {
        return file_name(&normalized);
    }
    normalized.bytes().any(|byte| byte.is_ascii_alphabetic())
        && segments.iter().all(|segment| path_segment(segment))
        && (segments.last().is_some_and(|name| file_name(name)) || segments.len() >= 3 || located)
}

fn path_segment(value: &str) -> bool {
    !value.is_empty()
        && !value.ends_with('.')
        && value
            .chars()
            .all(|ch| path_character(ch) && !matches!(ch, '/' | '\\' | ':' | '~'))
}

fn file_name(value: &str) -> bool {
    value.rsplit_once('.').is_some_and(|(base, extension)| {
        path_segment(base) && EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
    })
}

fn line_syntax(value: &str) -> bool {
    let mut ends = value.split('-');
    let valid = |part: &str| {
        let mut columns = part.split(':');
        columns.next().is_some_and(super::digits)
            && columns.next().is_none_or(super::digits)
            && columns.next().is_none()
    };
    ends.next().is_some_and(valid) && ends.next().is_none_or(valid) && ends.next().is_none()
}

#[cfg(test)]
mod tests;
