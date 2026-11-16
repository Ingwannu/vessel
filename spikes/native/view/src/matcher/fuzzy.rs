//! Opt-in history typo tier follows Paseo's per-token policy and ASCII word scans.
//! Whole-word marks avoid highlighting characters that a mistyped query lacks.
use super::{Field, Hit, MatchKind, Weight};

const EXACT_MAX_UNITS: usize = 3;
const SWAP_ONLY_UNITS: usize = 4;
const ONE_EDIT_MAX_UNITS: usize = 7;
const ONE_EDIT: usize = 1;
const TWO_EDITS: usize = 2;

pub(super) fn hit(field: &Field, token: &[char]) -> Option<Hit> {
    // Reference budgets count UTF-16 units, including non-ASCII query input.
    let query: Vec<u16> = token.iter().collect::<String>().encode_utf16().collect();
    if query.len() <= EXACT_MAX_UNITS {
        return None;
    }
    let budget = if query.len() <= ONE_EDIT_MAX_UNITS {
        ONE_EDIT
    } else {
        TWO_EDITS
    };
    let mut best: Option<Hit> = None;
    let mut offset = 0_usize;
    for run in field
        .0
        .split_inclusive(|letter| !letter.value.is_ascii_alphanumeric())
    {
        let word_length = run
            .iter()
            .take_while(|letter| letter.value.is_ascii_alphanumeric())
            .count();
        let word: Vec<u16> = run
            .iter()
            .take(word_length)
            .filter_map(|letter| u16::try_from(u32::from(letter.value)).ok())
            .collect();
        if let Some(distance) = word_distance(&query, &word, budget) {
            let candidate = Hit {
                kind: MatchKind::Fuzzy,
                weight: Weight {
                    tier: MatchKind::Fuzzy.tier(),
                    offset: field.offset(offset),
                    spread: distance,
                },
                span: offset..offset.saturating_add(word_length),
            };
            if best
                .as_ref()
                .is_none_or(|best| candidate.weight < best.weight)
            {
                best = Some(candidate);
            }
        }
        offset = offset.saturating_add(run.len());
    }
    best
}

fn word_distance(query: &[u16], word: &[u16], budget: usize) -> Option<usize> {
    // Typo prefixes compare to the full word and its two leading slices.
    [
        word.len(),
        query.len().min(word.len()),
        query.len().saturating_add(budget).min(word.len()),
    ]
    .into_iter()
    .filter_map(|length| {
        let candidate = word.get(..length)?;
        if query.len() == SWAP_ONLY_UNITS {
            swap_distance(query, candidate)
        } else {
            edit_distance(query, candidate, budget)
        }
    })
    .min()
}

fn swap_distance(query: &[u16], word: &[u16]) -> Option<usize> {
    if query.len() != word.len() {
        return None;
    }
    let first = query.iter().zip(word).position(|(one, two)| one != two)?;
    let second = first.saturating_add(1);
    (query.get(first) == word.get(second)
        && query.get(second) == word.get(first)
        && query.get(second.saturating_add(1)..) == word.get(second.saturating_add(1)..))
    .then_some(ONE_EDIT)
}

/// Three bounded rows implement Damerau-Levenshtein without a quadratic matrix.
/// Saturation preserves an over-budget result even at representational limits.
fn edit_distance(query: &[u16], word: &[u16], budget: usize) -> Option<usize> {
    if query.len().abs_diff(word.len()) > budget {
        return None;
    }
    let mut older = Vec::<usize>::new();
    let mut previous: Vec<usize> = (0..=word.len()).collect();
    for (query_at, query_unit) in query.iter().enumerate() {
        let row = query_at.saturating_add(1);
        let mut current = vec![row];
        let mut best = row;
        for (word_at, word_unit) in word.iter().enumerate() {
            let column = word_at.saturating_add(1);
            let replacement = usize::from(query_unit != word_unit);
            let mut cost = current
                .get(word_at)?
                .saturating_add(1)
                .min(previous.get(column)?.saturating_add(1))
                .min(previous.get(word_at)?.saturating_add(replacement));
            if let (Some(query_before), Some(word_before)) =
                (query_at.checked_sub(1), word_at.checked_sub(1))
                && query.get(query_before) == Some(word_unit)
                && word.get(word_before) == Some(query_unit)
            {
                cost = cost.min(older.get(word_before)?.saturating_add(1));
            }
            best = best.min(cost);
            current.push(cost);
        }
        if best > budget {
            return None;
        }
        older = previous;
        previous = current;
    }
    previous
        .last()
        .copied()
        .filter(|distance| *distance <= budget)
}
