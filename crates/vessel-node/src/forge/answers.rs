//! What the watcher does with its jobs' answers: a folder read puts a
//! session on a watch (or says why not), a look updates its watch and
//! every session on it, and a feedback job's signatures are kept; and
//! how long a watch waits after a good or a failed look (ADR 0029).

use std::time::Duration;

use chrono::Utc;
use tokio::time::Instant;
use vessel_forge::ForgeError;
use vessel_proto::SessionId;
use vessel_proto::forge::{ForgeAccess, PullState};

use super::feedback::{self, Feed};
use super::folder::Key;
use super::hosts::denied;
use super::poll::Polled;
use super::schedule::{
    BACKOFF_CAP, BACKOFF_FIRST, DENIED_AGAIN, Done, FIND_EVERY, FOLDER_EVERY, FolderRead,
    QUIET_AFTER, QUIET_EVERY, WATCH_EVERY,
};
use super::watcher::{Watch, Watcher};
use std::sync::Arc;

impl Watcher {
    /// A job's answer.
    pub(super) fn done(&mut self, done: Done) {
        match done {
            Done::Folder(read) => self.folder_read(*read),
            Done::Polled(answer) => {
                let (key, polled) = *answer;
                self.polled(&key, polled);
            }
            Done::Fed { session, sent } => {
                if let Some(held) = self.sessions.get_mut(&session) {
                    held.feeding = false;
                    held.sent = Some(sent);
                }
            }
        }
    }

    fn folder_read(&mut self, read: FolderRead) {
        let FolderRead {
            session,
            spot,
            remembered,
            access,
            feedback,
        } = read;
        self.folders = self.folders.saturating_sub(1);
        let jitter = self.jitter(FOLDER_EVERY);
        let Some(held) = self.sessions.get_mut(&session) else {
            return;
        };
        held.busy = false;
        held.remembered = remembered;
        held.due = Instant::now()
            .checked_add(jitter)
            .unwrap_or_else(Instant::now);
        if let Some(stored) = feedback {
            // A switch a person set meanwhile stands.
            let _known = held.feedback.get_or_insert(stored.on);
            if held.sent.is_none() {
                held.sent = Some(stored.sent.into_iter().collect());
            }
        }
        let why = match (spot, access) {
            (Ok(spot), Some(ForgeAccess::Ready { login })) => {
                self.attach(session, &spot, login);
                None
            }
            (Ok(_), Some(denied)) => Some(denied),
            (Err(why), _) => Some(why),
            // The job asks the host whenever it found a spot.
            (Ok(_), None) => Some(ForgeAccess::Unknown),
        };
        if let Some(why) = why {
            self.detach(session);
            if let Some(held) = self.sessions.get_mut(&session) {
                held.access = why;
            }
        }
        self.publish(session);
    }

    fn polled(&mut self, key: &Key, polled: Polled) {
        self.polls = self.polls.saturating_sub(1);
        let now = Instant::now();
        let Some(watch) = self.watches.get_mut(key) else {
            return;
        };
        watch.busy = false;
        watch.listed = polled.listed;
        if polled.found {
            watch.last_find = Some(now);
            if polled.number.is_none() && polled.error.is_none() {
                watch.known = None;
                watch.read = None;
                watch.seen = true;
            }
        }
        if let Some(read) = polled.read {
            watch.known = Some((
                read.pull.summary.number,
                read.pull.summary.head_commit.clone(),
            ));
            watch.read = Some(Arc::new(read));
            watch.last_full = Some(now);
            watch.seen = true;
        }
        if polled.changed {
            watch.changed_at = now;
        }
        let wait = match &polled.error {
            None => {
                watch.failures = 0;
                watch.error = None;
                interval(watch, now)
            }
            Some(error) => {
                watch.failures = watch.failures.saturating_add(1);
                watch.error = Some(denied(&key.0.host, error, watch.login.clone()));
                after_error(error, watch.failures)
            }
        };
        let wait = self.jitter(wait);
        let Some(watch) = self.watches.get_mut(key) else {
            return;
        };
        watch.due = now.checked_add(wait).unwrap_or(now);
        let replies = std::mem::take(&mut watch.replies);
        let sessions: Vec<SessionId> = watch.sessions.iter().copied().collect();
        for (session, reply) in replies {
            drop(reply.send(self.view(session)));
        }
        for session in sessions {
            self.publish(session);
            self.feed(session);
        }
    }

    /// Send `session` its pull request's new events, when its feedback is
    /// on and nothing is being sent to it already.
    pub(super) fn feed(&mut self, session: SessionId) {
        let Some(forge) = self.hosts.forge() else {
            return;
        };
        let Some(held) = self.sessions.get(&session) else {
            return;
        };
        if held.feedback != Some(true) || !held.structured || held.feeding {
            return;
        }
        let Some(read) = self.watch_of(held).and_then(|watch| watch.read.clone()) else {
            return;
        };
        let Some(sent) = held.sent.clone() else {
            return;
        };
        let feed = Feed {
            handle: held.handle.clone(),
            forge,
            store: self.store.clone(),
            read,
            sent,
            writers: Arc::clone(&self.writers),
        };
        if let Some(held) = self.sessions.get_mut(&session) {
            held.feeding = true;
        }
        drop(self.jobs.spawn(async move {
            Done::Fed {
                session,
                sent: feedback::feed(feed).await,
            }
        }));
    }
}

/// The wait after a good look.
fn interval(watch: &Watch, now: Instant) -> Duration {
    let landed = watch.read.as_ref().is_some_and(|read| {
        matches!(
            read.pull.summary.state,
            PullState::Merged | PullState::Closed
        )
    });
    if watch.known.is_none() {
        FIND_EVERY
    } else if landed || now.saturating_duration_since(watch.changed_at) >= QUIET_AFTER {
        QUIET_EVERY
    } else {
        WATCH_EVERY
    }
}

/// The wait after a failed look (the `failures`-th in a row).
fn after_error(error: &ForgeError, failures: u32) -> Duration {
    match error {
        ForgeError::RateLimited { until } => until
            .signed_duration_since(Utc::now())
            .to_std()
            .unwrap_or_default()
            .max(Duration::from_secs(1)),
        ForgeError::SignedOut { .. } | ForgeError::Refused { .. } => DENIED_AGAIN,
        ForgeError::Unreachable(_)
        | ForgeError::Down
        | ForgeError::Server { .. }
        | ForgeError::NotFound
        | ForgeError::Unexpected { .. }
        | ForgeError::Busy => {
            let doublings = failures.saturating_sub(1).min(16);
            BACKOFF_FIRST
                .saturating_mul(2_u32.saturating_pow(doublings))
                .min(BACKOFF_CAP)
        }
    }
}
