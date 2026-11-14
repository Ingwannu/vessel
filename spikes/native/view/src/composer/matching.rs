//! Ranked text matching, as Paseo ranks a picker's rows
//! (`packages/protocol/src/search/text-match.ts`: `scoreMatch`,
//! `scoreTextFields`, `compareMatchScores`), without its typo tolerance,
//! which Paseo's pickers leave off.
//!
//! A match is a tier and the place it was found; lower is better on both,
//! so callers sort ascending. Tiers, best first: the whole text, a whole
//! word, the text's start, a word's start, anywhere, then the query's
//! characters in order within one word.
//!
//! Positions count characters (Paseo counts UTF-16 units: the same order
//! for every text a command or a file name holds, only offsets past an
//! astral character differ, and only ties between such texts could sort
//! differently).

use std::cmp::Ordering;

/// How well a query matched a text (Paseo's `MatchScore`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchScore {
    tier: Tier,
    offset: usize,
    spread: usize,
}

/// Paseo's exact tiers, best first (its fuzzy tier is not used here).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Tier {
    Exact,
    WholeWord,
    Prefix,
    WordStart,
    Substring,
    Subsequence,
}

impl Tier {
    /// Its number in Paseo's sum over a query's tokens.
    const fn rank(self) -> usize {
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

impl Ord for MatchScore {
    fn cmp(&self, other: &Self) -> Ordering {
        self.tier
            .cmp(&other.tier)
            .then(self.offset.cmp(&other.offset))
            .then(self.spread.cmp(&other.spread))
    }
}

impl PartialOrd for MatchScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A token's summed score over a query (Paseo's aggregate: tiers, offsets
/// and spreads added up), compared the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldsScore {
    tier: usize,
    offset: usize,
    spread: usize,
}

fn lower(text: &str) -> Vec<char> {
    text.chars().flat_map(char::to_lowercase).collect()
}

/// Paseo's `isWordBoundaryChar`: anything but an ASCII lowercase letter or
/// digit (after lowercasing), the text's ends included.
fn boundary(character: Option<&char>) -> bool {
    !character.is_some_and(|character| matches!(character, 'a'..='z' | '0'..='9'))
}

fn substring(query: &[char], text: &[char]) -> Option<MatchScore> {
    if query.is_empty() || query.len() > text.len() {
        return None;
    }
    let mut best: Option<MatchScore> = None;
    for (found, window) in text.windows(query.len()).enumerate() {
        if window != query {
            continue;
        }
        let before = found.checked_sub(1).and_then(|at| text.get(at));
        let after = text.get(found.saturating_add(query.len()));
        let starts = found == 0 || boundary(before);
        let ends = after.is_none() || boundary(after);
        let tier = if starts && ends {
            Tier::WholeWord
        } else if found == 0 {
            Tier::Prefix
        } else if starts {
            Tier::WordStart
        } else {
            Tier::Substring
        };
        let score = MatchScore {
            tier,
            offset: found,
            spread: 0,
        };
        if best.is_none_or(|best| (tier, found) < (best.tier, best.offset)) {
            best = Some(score);
        }
    }
    best
}

/// The query's characters in order inside one whitespace-free run (Paseo
/// starts over at each space, so `labdes` does not join two words).
fn subsequence(query: &[char], text: &[char]) -> Option<MatchScore> {
    let mut at = 0_usize;
    let mut first: Option<usize> = None;
    let mut last = 0_usize;
    for (index, character) in text.iter().enumerate() {
        if at == query.len() {
            break;
        }
        if character.is_whitespace() {
            at = 0;
            first = None;
            continue;
        }
        if query.get(at) != Some(character) {
            continue;
        }
        let _first = first.get_or_insert(index);
        last = index;
        at = at.saturating_add(1);
    }
    let first = first.filter(|_| at == query.len())?;
    Some(MatchScore {
        tier: Tier::Subsequence,
        offset: first,
        spread: last.saturating_sub(first).saturating_add(1),
    })
}

/// How `query` matches `text`, case aside (Paseo's `scoreMatch`, exact
/// tiers and the subsequence); `None` when it does not.
#[must_use]
pub fn score_match(query: &str, text: &str) -> Option<MatchScore> {
    let exact = MatchScore {
        tier: Tier::Exact,
        offset: 0,
        spread: 0,
    };
    if query.is_empty() {
        return Some(exact);
    }
    let (query, text) = (lower(query), lower(text));
    if query == text {
        return Some(exact);
    }
    substring(&query, &text).or_else(|| subsequence(&query, &text))
}

/// How a query of space-separated tokens matches a row's texts (Paseo's
/// `scoreTextFields`): every token must match one of `fields`, each by its
/// best field; their scores add up. An empty query matches everything
/// equally.
#[must_use]
pub fn score_fields(query: &str, fields: &[&str]) -> Option<FieldsScore> {
    let mut total = FieldsScore {
        tier: 0,
        offset: 0,
        spread: 0,
    };
    for token in query.split_whitespace() {
        let best = fields
            .iter()
            .filter_map(|field| score_match(token, field))
            .min()?;
        // A substring hit has no spread of its own: Paseo counts the
        // token's length.
        let spread = match best.tier {
            Tier::Subsequence => best.spread,
            Tier::Exact | Tier::WholeWord | Tier::Prefix | Tier::WordStart | Tier::Substring => {
                token.chars().count()
            }
        };
        total.tier = total.tier.saturating_add(best.tier.rank());
        total.offset = total.offset.saturating_add(best.offset);
        total.spread = total.spread.saturating_add(spread);
    }
    Some(total)
}

#[cfg(test)]
mod tests;
