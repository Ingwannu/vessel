//! Contributions reuse the palette's shared text matcher. Section scores and
//! row scores are calculated in the same pass, so filtering cannot disagree
//! with ranking. Built-in recency bands remain in the existing ranking module.

use std::collections::BTreeMap;

use super::registration::{
    Contribution, GroupId, Presentation, Rank, Registry, ResultId, Visibility,
};
use crate::matcher::{Query, Score};

/// A matched contribution preserves its full typed target and source identity.
#[derive(Debug)]
pub struct ContributionResult<'a, T> {
    /// Stable cursor identity.
    pub id: ResultId,
    /// The source entry, borrowed until the next registration update.
    pub entry: &'a Contribution<T>,
    score: Score,
}

/// One visible section; the existing command renderer draws its rows.
#[derive(Debug)]
pub struct ContributionSection<'a, T> {
    /// Stable group identity.
    pub id: GroupId,
    /// Optional translated title.
    pub title: Option<&'a str>,
    /// Search-ranked rows, stable on equal scores.
    pub rows: Vec<ContributionResult<'a, T>>,
    rank: Rank,
    score: Score,
}

fn fields(presentation: &Presentation) -> Vec<&str> {
    match presentation {
        Presentation::Action { title, subtitle } => std::iter::once(title.as_str())
            .chain(subtitle.as_deref())
            .collect(),
        Presentation::Choice { path, selection: _ } => path.segments().collect(),
    }
}

/// Project registered actions; visible fields always outrank hidden synonyms.
/// Empty queries retain registry ordering and suppress query-only entries.
#[must_use]
pub fn contribution_sections<'a, T>(
    registry: &'a Registry<T>,
    text: &str,
) -> Vec<ContributionSection<'a, T>> {
    let query = Query::new(text);
    let mut sections: BTreeMap<GroupId, ContributionSection<'a, T>> = BTreeMap::new();
    for (id, entry) in registry.snapshot() {
        if query.is_empty() && entry.visibility == Visibility::Query {
            continue;
        }
        let hidden = entry.keywords.join(" ");
        let Some(score) = query.score(&fields(&entry.presentation), Some(&hidden)) else {
            continue;
        };
        let section = sections
            .entry(entry.group.clone())
            .or_insert_with(|| ContributionSection {
                id: entry.group.clone(),
                title: entry.section_title.as_deref(),
                rows: Vec::new(),
                rank: entry.group_rank,
                score,
            });
        section.score = section.score.min(score);
        section.rows.push(ContributionResult { id, entry, score });
    }
    let mut sections: Vec<_> = sections.into_values().collect();
    if query.is_empty() {
        sections.sort_by(|left, right| (left.rank, &left.id).cmp(&(right.rank, &right.id)));
    } else {
        for section in &mut sections {
            section.rows.sort_by_key(|row| row.score);
        }
        sections.sort_by(|left, right| {
            (left.score, left.rank, &left.id).cmp(&(right.score, right.rank, &right.id))
        });
    }
    sections
}

#[cfg(test)]
mod tests;
