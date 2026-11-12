//! One look at a watch's pull request: find it by its branch when it is
//! not known (or landed), ask whether it changed, and read it whole when
//! it did or the last whole read is old (ADR 0029, "How the node
//! watches"). A task of its own; its answer goes back to the watcher.
//!
//! [Decision Log]
//! - Purpose: the cheap path, nothing changed, must cost three 304s and
//!   no GraphQL; a whole read only when something moved.
//! - The pick (Paseo's): the most recently updated open pull request of
//!   the branch; else the most recently updated closed or merged one whose
//!   head is the folder's commit now, so an old pull request of a reused
//!   branch name is not shown for new work.

use std::sync::Arc;

use vessel_forge::{Forge, ForgeError, Found, Fresh, Priority, Read};
use vessel_proto::forge::{PullNumber, PullState, RepoRef};
use vessel_proto::workspace::{BranchName, CommitId};

/// What a look needs.
#[derive(Debug)]
pub(crate) struct Ask {
    pub(crate) forge: Arc<dyn Forge>,
    pub(crate) repo: RepoRef,
    pub(crate) branch: BranchName,
    /// The commit a session's folder has checked out (for the pick).
    pub(crate) head: CommitId,
    /// The branch's pull requests as last listed.
    pub(crate) listed: Vec<Found>,
    /// The pull request last read, and its head then.
    pub(crate) known: Option<(PullNumber, CommitId)>,
    /// List the branch's pull requests again.
    pub(crate) find: bool,
    /// Read whole even when nothing changed.
    pub(crate) whole: bool,
    pub(crate) priority: Priority,
}

/// What a look found.
#[derive(Debug)]
pub(crate) struct Polled {
    /// The branch's pull requests, as listed now (or as before).
    pub(crate) listed: Vec<Found>,
    /// The list was asked for.
    pub(crate) found: bool,
    /// The pull request picked; `None` when the branch has none to show.
    pub(crate) number: Option<PullNumber>,
    /// It was read whole: its facts and discussion.
    pub(crate) read: Option<Read>,
    /// Something changed since the last look.
    pub(crate) changed: bool,
    /// A request failed; what came before it is kept.
    pub(crate) error: Option<ForgeError>,
}

/// Look once.
pub(crate) async fn poll(ask: Ask) -> Polled {
    let mut polled = Polled {
        listed: ask.listed.clone(),
        found: false,
        number: ask.known.as_ref().map(|(number, _)| *number),
        read: None,
        changed: false,
        error: None,
    };
    if ask.find || ask.known.is_none() {
        match ask.forge.find(&ask.repo, &ask.branch, ask.priority).await {
            Ok(Fresh::Changed(listed)) => {
                polled.listed = listed;
                polled.changed = true;
            }
            Ok(Fresh::Same) => {}
            Err(error) => {
                polled.error = Some(error);
                return polled;
            }
        }
        polled.found = true;
        polled.number = pick(&polled.listed, &ask.head).map(|found| found.number);
    }
    let Some(number) = polled.number else {
        return polled;
    };
    let moved = match &ask.known {
        Some((known, head)) if *known == number => {
            match ask
                .forge
                .changed(&ask.repo, number, head, ask.priority)
                .await
            {
                Ok(changed) => changed,
                Err(error) => {
                    polled.error = Some(error);
                    return polled;
                }
            }
        }
        Some(_) | None => true,
    };
    polled.changed |= moved;
    if moved || ask.whole {
        match ask.forge.read(&ask.repo, number, ask.priority).await {
            Ok(read) => polled.read = Some(read),
            Err(error) => polled.error = Some(error),
        }
    }
    polled
}

/// The pull request that is the branch's now (the module's rule).
pub(crate) fn pick<'a>(listed: &'a [Found], head: &CommitId) -> Option<&'a Found> {
    let newest = |state: fn(PullState) -> bool, also: &dyn Fn(&Found) -> bool| {
        listed
            .iter()
            .filter(|found| state(found.state) && also(found))
            .max_by_key(|found| found.updated_at)
    };
    newest(|state| state == PullState::Open, &|_| true).or_else(|| {
        newest(
            |state| matches!(state, PullState::Merged | PullState::Closed),
            &|found| found.head == *head,
        )
    })
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;

    fn found(number: u64, state: PullState, head: char, at: i64) -> Found {
        Found {
            number: PullNumber::new(number).unwrap(),
            state,
            head: head.to_string().repeat(40).parse().unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(at, 0).unwrap(),
        }
    }

    #[test]
    fn the_newest_open_one_else_a_landed_one_of_this_head() {
        let head: CommitId = "a".repeat(40).parse().unwrap();
        let listed = [
            found(1, PullState::Merged, 'a', 5),
            found(2, PullState::Open, 'b', 1),
            found(3, PullState::Open, 'c', 2),
        ];
        assert_eq!(
            pick(&listed, &head).map(|found| found.number.get()),
            Some(3)
        );
        let landed = [
            found(1, PullState::Merged, 'b', 9),
            found(4, PullState::Closed, 'a', 3),
        ];
        assert_eq!(
            pick(&landed, &head).map(|found| found.number.get()),
            Some(4)
        );
        let old = [found(1, PullState::Merged, 'b', 9)];
        assert_eq!(pick(&old, &head), None, "an old one of a reused name");
    }
}
