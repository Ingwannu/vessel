//! When the watcher reads a folder or looks at a watch, and the jobs it
//! starts for them (ADR 0029's intervals, backoff and cooldowns; what it
//! does with their answers is `answers`').
//!
//! [Decision Log]
//! - Purpose: a watched pull request should be fresh within a minute while
//!   it moves, and cost next to nothing while it does not; a forge that
//!   fails or limits the node must be left alone for a while.
//! - Choice: a folder every [`FOLDER_EVERY`]; a watch every
//!   [`WATCH_EVERY`], [`QUIET_EVERY`] once nothing changed for
//!   [`QUIET_AFTER`] or its pull request landed, and [`FIND_EVERY`] while
//!   none is found; each jittered by up to a tenth so watches do not move
//!   in step. A failed look backs its watch off from [`BACKOFF_FIRST`]
//!   doubling to [`BACKOFF_CAP`]; a rate limit until its end; a refusal or
//!   a missing sign-in five minutes. The last facts are kept meanwhile.

use std::collections::HashSet;
use std::hash::BuildHasher;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::Instant;
use vessel_forge::Priority;
use vessel_proto::SessionId;
use vessel_proto::forge::{ForgeAccess, PullState};
use vessel_store::StoredFeedback;

use super::folder::{self, Key, Remembered, Spot};
use super::poll::{self, Ask, Polled};
use super::watcher::{FOLDERS, POLLS, Watcher};

pub(super) const FOLDER_EVERY: Duration = Duration::from_secs(60);
pub(super) const WATCH_EVERY: Duration = Duration::from_secs(60);
pub(super) const QUIET_EVERY: Duration = Duration::from_secs(300);
pub(super) const QUIET_AFTER: Duration = Duration::from_secs(30 * 60);
pub(super) const FIND_EVERY: Duration = Duration::from_secs(120);
/// A whole read at least this often, even when no request noticed a change.
const FULL_EVERY: Duration = Duration::from_secs(300);
pub(super) const BACKOFF_FIRST: Duration = Duration::from_secs(60);
pub(super) const BACKOFF_CAP: Duration = Duration::from_secs(30 * 60);
/// Signed out or refused: asked again this much later.
pub(super) const DENIED_AGAIN: Duration = Duration::from_secs(300);

/// A folder read's answer.
#[derive(Debug)]
pub(super) struct FolderRead {
    pub(super) session: SessionId,
    pub(super) spot: Result<Spot, ForgeAccess>,
    pub(super) remembered: Option<Remembered>,
    /// The host's access, when a spot was found.
    pub(super) access: Option<ForgeAccess>,
    /// The store's feedback, on the session's first read.
    pub(super) feedback: Option<StoredFeedback>,
}

/// A job's answer.
#[derive(Debug)]
pub(super) enum Done {
    /// Boxed, as `Polled`: answers are much larger than a feedback
    /// job's.
    Folder(Box<FolderRead>),
    Polled(Box<(Key, Polled)>),
    Fed {
        session: SessionId,
        sent: HashSet<String>,
    },
}

impl Watcher {
    /// Start every job that is due and fits.
    pub(super) fn start_due(&mut self, now: Instant) {
        let ended: Vec<SessionId> = self
            .sessions
            .iter()
            .filter(|(_, held)| held.handle.has_ended())
            .map(|(id, _)| *id)
            .collect();
        for session in ended {
            self.forget(session);
        }
        let due: Vec<SessionId> = self
            .sessions
            .iter()
            .filter(|(_, held)| !held.busy && held.due <= now)
            .map(|(id, _)| *id)
            .collect();
        for session in due {
            if self.folders >= FOLDERS {
                break;
            }
            self.read_folder(session);
        }
        let due: Vec<Key> = self
            .watches
            .iter()
            .filter(|(_, watch)| !watch.busy && watch.due <= now)
            .map(|(key, _)| key.clone())
            .collect();
        for key in due {
            if self.polls >= POLLS {
                break;
            }
            self.look(&key);
        }
    }

    /// When the next job is due, among those that would fit now.
    pub(super) fn next_due(&self) -> Instant {
        let idle = Instant::now()
            .checked_add(WATCH_EVERY)
            .unwrap_or_else(Instant::now);
        let folders = (self.folders < FOLDERS)
            .then(|| {
                self.sessions
                    .values()
                    .filter(|held| !held.busy)
                    .map(|held| held.due)
                    .min()
            })
            .flatten();
        let watches = (self.polls < POLLS)
            .then(|| {
                self.watches
                    .values()
                    .filter(|watch| !watch.busy)
                    .map(|watch| watch.due)
                    .min()
            })
            .flatten();
        [folders, watches]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(idle)
            .min(idle)
    }

    fn read_folder(&mut self, session: SessionId) {
        let Some(held) = self.sessions.get_mut(&session) else {
            return;
        };
        held.busy = true;
        self.folders = self.folders.saturating_add(1);
        let workspaces = Arc::clone(&self.workspaces);
        let hosts = Arc::clone(&self.hosts);
        let store = self.store.clone();
        let folder = held.folder.clone();
        let project = held.project;
        let remembered = held.remembered.clone();
        let load = held.sent.is_none();
        drop(self.jobs.spawn(async move {
            let (spot, remembered) = folder::read(&workspaces, &folder, project, remembered).await;
            let access = match &spot {
                Ok(spot) => Some(hosts.access(&spot.repo.host).await),
                Err(_) => None,
            };
            let feedback = if load {
                match store.feedback(session).await {
                    Ok(feedback) => Some(feedback),
                    Err(error) => {
                        tracing::warn!(%session, %error, "feedback could not be read");
                        None
                    }
                }
            } else {
                None
            };
            Done::Folder(Box::new(FolderRead {
                session,
                spot,
                remembered,
                access,
                feedback,
            }))
        }));
    }

    fn look(&mut self, key: &Key) {
        let Some(forge) = self.hosts.forge() else {
            return;
        };
        let Some(watch) = self.watches.get_mut(key) else {
            return;
        };
        let head = watch
            .sessions
            .iter()
            .find_map(|session| self.sessions.get(session)?.head.clone());
        let Some(head) = head else {
            return;
        };
        let now = Instant::now();
        let landed = watch.read.as_ref().is_some_and(|read| {
            matches!(
                read.pull.summary.state,
                PullState::Merged | PullState::Closed
            )
        });
        let ask = Ask {
            forge,
            repo: key.0.clone(),
            branch: key.1.clone(),
            head,
            listed: watch.listed.clone(),
            known: watch.known.clone(),
            find: landed && older(watch.last_find, now, QUIET_EVERY),
            whole: older(watch.last_full, now, FULL_EVERY),
            priority: if watch.urgent {
                Priority::Person
            } else {
                Priority::Background
            },
        };
        watch.busy = true;
        watch.urgent = false;
        self.polls = self.polls.saturating_add(1);
        let key = key.clone();
        drop(
            self.jobs
                .spawn(async move { Done::Polled(Box::new((key, poll::poll(ask).await))) }),
        );
    }

    /// `base`, less up to a tenth (so watches spread out, and never later
    /// than asked).
    pub(super) fn jitter(&self, base: Duration) -> Duration {
        let seed = self.random.hash_one(Instant::now());
        let tenth = base.checked_div(10).unwrap_or_default();
        let millis = u64::try_from(tenth.as_millis()).unwrap_or(u64::MAX);
        let off = seed.checked_rem(millis.saturating_add(1)).unwrap_or(0);
        base.saturating_sub(Duration::from_millis(off))
    }
}

/// Whether `at` is unset or at least `every` before `now`.
fn older(at: Option<Instant>, now: Instant, every: Duration) -> bool {
    at.is_none_or(|at| now.saturating_duration_since(at) >= every)
}
