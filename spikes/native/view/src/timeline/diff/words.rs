//! The source's adjacent remove/add word diff, bounded independently of line count.
//! ASCII word/nonword runs match JavaScript's non-Unicode `\w` behavior. Ranges
//! refer to UTF-8 bytes of the original line; text itself is never reconstructed.
use super::{DiffLine, MAX_LINES, Side};
use std::ops::Range;

/// One long generated line keeps its ordinary diff without a costly word table.
const MAX_WORD_BYTES: usize = 16_384;
const MAX_WORD_TOKENS: usize = 256;
/// Shared work budget for all adjacent pairs in one bounded diff projection.
const MAX_WORD_CELLS: usize = 1_048_576;

/// Changed byte ranges, in the same order as the supplied diff's rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordHighlights(Vec<Vec<Range<usize>>>);
impl WordHighlights {
    /// Missing/out-of-range rows have ordinary whole-line presentation.
    #[must_use]
    pub fn for_line(&self, line: usize) -> &[Range<usize>] {
        self.0.get(line).map_or(&[], Vec::as_slice)
    }
}

/// Match the reference's adjacent removed/added pair only, not a guessed
/// alignment of every line in a replacement block. Exhaustion retains line tones.
#[must_use]
pub fn word_highlights(lines: &[DiffLine<'_>]) -> WordHighlights {
    let mut highlights = vec![Vec::new(); lines.len().min(MAX_LINES)];
    let mut budget = MAX_WORD_CELLS;
    for (index, pair) in lines
        .windows(2)
        .take(MAX_LINES.saturating_sub(1))
        .enumerate()
    {
        if budget == 0 {
            break;
        }
        let Some((old, new)) = pair.first().zip(pair.get(1)) else {
            continue;
        };
        if old.side != Side::Removed || new.side != Side::Added {
            continue;
        }
        if let Some((removed, added)) = changed_pair(old.text, new.text, &mut budget) {
            if let Some(target) = highlights.get_mut(index) {
                *target = removed;
            }
            if let Some(target) = highlights.get_mut(index.saturating_add(1)) {
                *target = added;
            }
        }
    }
    WordHighlights(highlights)
}

fn words(text: &str) -> Option<Vec<&str>> {
    if text.len() > MAX_WORD_BYTES {
        return None;
    }
    let mut words = Vec::new();
    let mut start = 0;
    let mut previous = None;
    for (at, character) in text.char_indices() {
        let word = character.is_ascii_alphanumeric() || character == '_';
        if previous.is_some_and(|before| before != word) {
            words.push(text.get(start..at)?);
            if words.len() >= MAX_WORD_TOKENS {
                return None;
            }
            start = at;
        }
        previous = Some(word);
    }
    if !text.is_empty() {
        words.push(text.get(start..)?);
    }
    Some(words)
}

fn index(width: usize, row: usize, column: usize) -> Option<usize> {
    row.checked_mul(width)?.checked_add(column)
}
fn at(grid: &[usize], width: usize, row: usize, column: usize) -> Option<usize> {
    grid.get(index(width, row, column)?).copied()
}

fn changed_pair(
    old: &str,
    new: &str,
    budget: &mut usize,
) -> Option<(Vec<Range<usize>>, Vec<Range<usize>>)> {
    let old = words(old)?;
    let new = words(new)?;
    let width = new.len().checked_add(1)?;
    let cells = old.len().checked_add(1)?.checked_mul(width)?;
    if cells > *budget {
        return None;
    }
    *budget = budget.saturating_sub(cells);
    let mut grid = vec![0; cells];
    for (i, before) in old.iter().enumerate().rev() {
        for (j, after) in new.iter().enumerate().rev() {
            let next_i = i.checked_add(1)?;
            let next_j = j.checked_add(1)?;
            let value = if before == after {
                at(&grid, width, next_i, next_j)?.checked_add(1)?
            } else {
                at(&grid, width, next_i, j)?.max(at(&grid, width, i, next_j)?)
            };
            *grid.get_mut(index(width, i, j)?)? = value;
        }
    }
    let mut old_kept = vec![false; old.len()];
    let mut new_kept = vec![false; new.len()];
    let (mut i, mut j) = (0_usize, 0_usize);
    while let Some((before, after)) = old.get(i).zip(new.get(j)) {
        let next_i = i.checked_add(1)?;
        let next_j = j.checked_add(1)?;
        if before == after {
            *old_kept.get_mut(i)? = true;
            *new_kept.get_mut(j)? = true;
            (i, j) = (next_i, next_j);
        } else if at(&grid, width, next_i, j)? >= at(&grid, width, i, next_j)? {
            i = next_i;
        } else {
            j = next_j;
        }
    }
    Some((ranges(&old, &old_kept), ranges(&new, &new_kept)))
}

fn ranges(words: &[&str], kept: &[bool]) -> Vec<Range<usize>> {
    let mut ranges: Vec<Range<usize>> = Vec::new();
    let mut start = 0_usize;
    for (word, same) in words.iter().zip(kept) {
        let end = start.saturating_add(word.len());
        if !same {
            if let Some(last) = ranges.last_mut()
                && last.end == start
            {
                last.end = end;
            } else {
                ranges.push(start..end);
            }
        }
        start = end;
    }
    ranges
}

#[cfg(test)]
mod tests;
