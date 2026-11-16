//! Ranked matching shared by commands and provider catalogs, specified
//! by Paseo's protocol text matcher and `provider-catalog-search.ts`.
//!
//! [Decision Log]
//! - Purpose: a query such as `lbl dsgn` must find Label as Design, with
//!   visible matches ahead of hidden keywords and honest highlights.
//! - Constraint: the Command Center enables subsequences, not edit-distance
//!   typo correction. Characters cannot be joined across whitespace.
//! - Choice: standard-library token scans with ordered scores. UTF-16
//!   offsets preserve reference ranking; UTF-8 ranges address Rust text.
//! - Alternative: a general fuzzy-search dependency has different scoring
//!   and would make reference parity harder. No dependency is added.
//! - Cost: each candidate is folded for the scan; highlight ranges are
//!   computed only for rows being drawn, not while ranking results.
//! - Sharing: one matcher serves every consumer; its opaque ordered score
//!   keeps callers from inventing incompatible relevance weights.

use std::cmp::Ordering;
use std::ops::Range;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
struct Weight {
    tier: usize,
    offset: usize,
    spread: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MatchKind {
    Exact,
    WholeWord,
    Prefix,
    WordStart,
    Substring,
    Subsequence,
}

impl MatchKind {
    const fn tier(self) -> usize {
        match self {
            Self::Exact => 0,
            Self::WholeWord => 1,
            Self::Prefix => 2,
            Self::WordStart => 3,
            Self::Substring => 4,
            Self::Subsequence => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum FieldRole {
    Visible,
    Keyword,
}

/// Total ranking: visible fields before hidden keywords, then token
/// match tier, UTF-16 offset and spread. Its fields are intentionally
/// opaque; callers compare scores without inventing another scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score {
    role: FieldRole,
    weight: Weight,
}

impl Score {
    pub(crate) fn built_in_cmp(self, other: Self) -> Ordering {
        (self.role, self.weight.tier).cmp(&(other.role, other.weight.tier))
    }
}

struct Letter {
    value: char,
    bytes: Range<usize>,
    utf16: usize,
}

struct Field(Vec<Letter>);

impl Field {
    fn new(text: &str) -> Self {
        let spans = text.char_indices().flat_map(|(at, value)| {
            value
                .to_lowercase()
                .map(move |_| at..at.saturating_add(value.len_utf8()))
        });
        let mut utf16 = 0_usize;
        Self(
            text.to_lowercase()
                .chars()
                .zip(spans)
                .map(|(value, bytes)| {
                    let letter = Letter {
                        value,
                        bytes,
                        utf16,
                    };
                    utf16 = utf16.saturating_add(value.len_utf16());
                    letter
                })
                .collect(),
        )
    }

    fn offset(&self, at: usize) -> usize {
        self.0.get(at).map_or_else(
            || {
                self.0
                    .last()
                    .map_or(0, |last| last.utf16.saturating_add(last.value.len_utf16()))
            },
            |letter| letter.utf16,
        )
    }

    fn hit(&self, token: &[char]) -> Option<Hit> {
        if token.is_empty() {
            return None;
        }
        let length = token.iter().fold(0_usize, |count, value| {
            count.saturating_add(value.len_utf16())
        });
        let contiguous = self
            .0
            .windows(token.len())
            .enumerate()
            .filter_map(|(start, run)| {
                if !run
                    .iter()
                    .map(|letter| letter.value)
                    .eq(token.iter().copied())
                {
                    return None;
                }
                let end = start.saturating_add(token.len());
                let boundary = |letter: Option<&Letter>| {
                    letter.is_none_or(|letter| !letter.value.is_ascii_alphanumeric())
                };
                let left = boundary(start.checked_sub(1).and_then(|at| self.0.get(at)));
                let right = boundary(self.0.get(end));
                let kind = if start == 0 && end == self.0.len() {
                    MatchKind::Exact
                } else if left && right {
                    MatchKind::WholeWord
                } else if start == 0 {
                    MatchKind::Prefix
                } else if left {
                    MatchKind::WordStart
                } else {
                    MatchKind::Substring
                };
                Some(Hit {
                    kind,
                    weight: Weight {
                        tier: kind.tier(),
                        offset: self.offset(start),
                        spread: length,
                    },
                    span: start..end,
                })
            })
            .min_by_key(|hit| hit.weight);
        contiguous.or_else(|| self.subsequence(token))
    }

    fn subsequence(&self, token: &[char]) -> Option<Hit> {
        let mut matched = 0_usize;
        let mut first = 0;
        for (at, letter) in self.0.iter().enumerate() {
            if whitespace(letter.value) {
                matched = 0;
            } else if token.get(matched) == Some(&letter.value) {
                if matched == 0 {
                    first = at;
                }
                matched = matched.saturating_add(1);
                if matched == token.len() {
                    let end = at.saturating_add(1);
                    return Some(Hit {
                        kind: MatchKind::Subsequence,
                        weight: Weight {
                            tier: 5,
                            offset: self.offset(first),
                            spread: self.offset(end).saturating_sub(self.offset(first)),
                        },
                        span: first..end,
                    });
                }
            }
        }
        None
    }
}

struct Hit {
    kind: MatchKind,
    weight: Weight,
    span: Range<usize>,
}

pub(crate) fn whitespace(value: char) -> bool {
    // ECMAScript's \s includes BOM but excludes Unicode's next-line
    // character. Rust's is_whitespace would split different words.
    matches!(
        value,
        '\u{0009}'..='\u{000d}'
            | '\u{0020}'
            | '\u{00a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    )
}

/// Parsed once for a result projection, rather than once per candidate.
#[derive(Debug)]
pub struct Query(Vec<Vec<char>>);

impl Query {
    /// Parse case-folded, whitespace-separated query tokens.
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self(
            text.to_lowercase()
                .split(whitespace)
                .filter(|token| !token.is_empty())
                .map(|token| token.chars().collect())
                .collect(),
        )
    }

    /// Whether the query contains no tokens.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn fields(&self, fields: &[Field]) -> Option<Weight> {
        let mut total = Weight::default();
        for token in &self.0 {
            let best = fields
                .iter()
                .filter_map(|field| field.hit(token))
                .map(|hit| hit.weight)
                .min()?;
            // Reference scores sum per-token matches. Saturation preserves
            // ordering at the representable bound instead of overflowing.
            total.tier = total.tier.saturating_add(best.tier);
            total.offset = total.offset.saturating_add(best.offset);
            total.spread = total.spread.saturating_add(best.spread);
        }
        Some(total)
    }

    /// Score every token against visible fields, falling back to the
    /// visible fields plus hidden keywords only when a visible match is
    /// impossible. Subsequence matching stays inside each word; typo
    /// tolerance is disabled, as in Paseo's Command Center and catalog.
    #[must_use]
    pub fn score(&self, visible: &[&str], hidden: Option<&str>) -> Option<Score> {
        let mut fields: Vec<_> = visible.iter().map(|text| Field::new(text)).collect();
        if let Some(weight) = self.fields(&fields) {
            return Some(Score {
                role: FieldRole::Visible,
                weight,
            });
        }
        fields.push(Field::new(hidden?));
        self.fields(&fields).map(|weight| Score {
            role: FieldRole::Keyword,
            weight,
        })
    }

    /// Matching tokens and scattered runs in original UTF-8 bytes.
    /// Overlaps share one mark; nonmatching tokens add no ranges.
    #[must_use]
    pub fn marks(&self, text: &str) -> Vec<Range<usize>> {
        let field = Field::new(text);
        let mut ranges = Vec::new();
        for token in &self.0 {
            let Some(hit) = field.hit(token) else {
                continue;
            };
            let mut wanted = token.iter();
            let mut next = wanted.next();
            for letter in field.0.get(hit.span).into_iter().flatten() {
                if hit.kind != MatchKind::Subsequence || next == Some(&letter.value) {
                    ranges.push(letter.bytes.clone());
                    if hit.kind == MatchKind::Subsequence {
                        next = wanted.next();
                    }
                }
            }
        }
        ranges.sort_by_key(|range| (range.start, range.end));
        let mut merged: Vec<Range<usize>> = Vec::new();
        for range in ranges {
            if let Some(last) = merged.last_mut()
                && range.start <= last.end
            {
                last.end = last.end.max(range.end);
            } else {
                merged.push(range);
            }
        }
        merged
    }
}
