//! Repository pages from the same fake state used by reads and merges.
//! File fixtures already bound that state; this does not create watches
//! or a second collection of simulated pull requests.

use chrono::Utc;
use vessel_proto::forge::{OpenPullPage, PullCursor, PullNumber, PullState, RepoRef};

use super::FakeForge;
use crate::provider::ForgeError;

const CURSOR_PREFIX: &str = "fixture:";

impl FakeForge {
    pub(super) fn project_page(
        &self,
        repo: &RepoRef,
        after: Option<&PullCursor>,
    ) -> Result<OpenPullPage, ForgeError> {
        let state = self.enter_repo("list_open", repo)?;
        let mut listed: Vec<_> = state
            .pulls
            .values()
            .map(|pull| &pull.read.pull.summary)
            .filter(|summary| &summary.repo == repo && summary.state == PullState::Open)
            .collect();
        listed.sort_by(|a, b| {
            b.updated_at
                .cmp(&a.updated_at)
                .then_with(|| b.number.cmp(&a.number))
        });
        let start =
            match after {
                None => 0,
                Some(cursor) => {
                    let text = cursor.as_str().strip_prefix(CURSOR_PREFIX).ok_or(
                        ForgeError::Unexpected {
                            what: "a fixture project cursor",
                        },
                    )?;
                    let number = PullNumber::new(text.parse().map_err(ForgeError::ListingInteger)?)
                        .map_err(ForgeError::ListingValue)?;
                    // Advance once past the exact item. Overflow refuses the
                    // whole read rather than wrapping back to the first page.
                    listed
                        .iter()
                        .position(|pull| pull.number == number)
                        .and_then(|index| index.checked_add(1))
                        .ok_or(ForgeError::Unexpected {
                            what: "a fixture cursor still in its open list",
                        })?
                }
            };
        let has_more = listed
            .iter()
            .skip(start)
            .nth(OpenPullPage::MAX_PULLS)
            .is_some();
        let pulls: Vec<_> = listed
            .iter()
            .skip(start)
            .take(OpenPullPage::MAX_PULLS)
            .copied()
            .cloned()
            .collect();
        let next = if has_more {
            let last = pulls.last().ok_or(ForgeError::Unexpected {
                what: "a nonempty fixture page",
            })?;
            Some(
                format!("{CURSOR_PREFIX}{}", last.number)
                    .parse()
                    .map_err(ForgeError::PageCursor)?,
            )
        } else {
            None
        };
        OpenPullPage::new(repo.clone(), pulls, next, Utc::now()).map_err(ForgeError::Page)
    }
}

#[cfg(test)]
mod tests;
