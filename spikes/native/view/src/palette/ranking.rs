//! The Command Center's two ordering bands: commands by relevance,
//! then session navigation by match tier and recency. Equal command
//! scores retain registry order rather than jumping alphabetically.

use std::cmp::Ordering;

use super::{PaletteEntry, PaletteGroup};
use crate::matcher::{Query, Score};

type Group<'a, S, T> = (PaletteGroup, Vec<&'a PaletteEntry<S, T>>);

/// Paseo shows only five recent built-in rows before a query narrows them.
const RECENT_LIMIT: usize = 5;

pub(super) fn groups<'a, S, T>(
    entries: &'a [PaletteEntry<S, T>],
    query: &str,
) -> Vec<Group<'a, S, T>> {
    let query = Query::new(query);
    let mut populated: Vec<(Option<Score>, Group<'a, S, T>)> = PaletteGroup::ALL
        .into_iter()
        .filter_map(|group| {
            let mut scored: Vec<_> = entries
                .iter()
                .filter(|entry| entry.group == group)
                .filter_map(|entry| entry.score(&query).map(|score| (score, entry)))
                .collect();
            scored.sort_by(|(left_score, left), (right_score, right)| {
                if group == PaletteGroup::Sessions {
                    left_score
                        .built_in_cmp(*right_score)
                        .then_with(|| right.activity_at.cmp(&left.activity_at))
                } else {
                    left_score.cmp(right_score)
                }
            });
            let best = scored.iter().map(|(score, _)| *score).min();
            let mut found: Vec<_> = scored.into_iter().map(|(_, entry)| entry).collect();
            if group == PaletteGroup::Sessions && query.is_empty() {
                found.truncate(RECENT_LIMIT);
            }
            (!found.is_empty()).then_some((best, (group, found)))
        })
        .collect();
    populated.sort_by(|(left_score, (left, _)), (right_score, (right, _))| {
        let bands = (*left == PaletteGroup::Sessions).cmp(&(*right == PaletteGroup::Sessions));
        bands.then_with(|| {
            if *left == PaletteGroup::Sessions || query.is_empty() {
                Ordering::Equal
            } else {
                left_score.cmp(right_score)
            }
        })
    });
    populated.into_iter().map(|(_, group)| group).collect()
}
