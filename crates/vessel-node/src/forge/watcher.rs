//! The one task that watches pull requests (ADR 0029, "How the node
//! watches"): which sessions it holds, the watches they share, what each
//! is given, and the commands calls send it. Its timing is `schedule`'s,
//! the answers of its jobs `answers`', putting sessions on watches
//! `watches`'.
//!
//! [Decision Log]
//! - Purpose: many sessions may sit on one branch, and every look costs
//!   requests against a forge's quota; one owner keeps the state, so no
//!   lock is held across a request and no two looks at one branch run.
//! - Choice: sessions are read each cycle for their folder (at most
//!   [`FOLDERS`] git reads at once), and attached to a watch keyed by
//!   repository and branch; at most [`MAX_WATCHES`] watches, sessions a
//!   person started before an agent's (an agent's watch is given up for a
//!   person's). Every look and read is a job of its own whose answer comes
//!   back here; nothing here awaits the network.
//! - A session leaves when it is archived or deleted (told), or when its
//!   handle says it ended (seen at its next turn in the schedule).
//!
//! [Decision Log] A refresh before a session's first folder read
//! - Purpose: a session starts on no watch until its folder is read (its
//!   branch and remote), and that read is a job of its own. A person's
//!   refresh that came before it was answered at once with the empty view,
//!   so the card said "no pull request" while one existed, and the refresh
//!   stamped `refreshed`, so the next one within 10 s was held to that
//!   empty view too. A loaded node made this common (the e2e
//!   `a_forge_that_is_down_holds_nothing_and_an_end_clears_it` lost it).
//! - Choice: until the first read is done (`read_once`) a refresh waits on
//!   the session (`waiting`, at most [`MAX_WAITING`]; past that it is
//!   answered with the view as it is) and stamps nothing. The read answers
//!   them: on a watch they join its next look, made now, as a refresh
//!   does; on none they get the view, which says why (no branch, no
//!   remote, signed out). A session that leaves answers them `None`.
//! - Bounded: every job's task id names what it was for (`running`), so a
//!   job that panics still frees its slot and answers whoever waited on
//!   it (a folder read's or a look's refreshes) with the view as it is;
//!   before, such a job left its session or watch busy for good.
//! - Alternatives: reading the folder inside the refresh call (a second
//!   reader racing the watcher's own); answering "not read yet" (the
//!   person asks again for no reason).

use std::collections::hash_map::RandomState;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use vessel_forge::{Found, Read};
use vessel_proto::forge::{
    ForgeAccess, ForgeLogin, PullNumber, PullRequestList, PullRequestView, SessionPullEntry,
    SessionPullRequest,
};
use vessel_proto::workspace::CommitId;
use vessel_proto::{ProjectId, SessionId};
use vessel_session::SessionHandle;
use vessel_store::Store;

use super::feedback::Writers;
use super::folder::{Key, Remembered};
use super::hosts::Hosts;
use super::schedule::Done;
use super::{Command, Started, view};
use crate::workspace::Workspaces;

/// The most watches a node keeps (the contract's bound).
pub(crate) const MAX_WATCHES: usize = 64;
/// Git reads of the watcher's at once.
pub(super) const FOLDERS: usize = 2;
/// Looks at once (each a few requests; the HTTP client has its own
/// slots).
pub(super) const POLLS: usize = 4;
/// Refreshes one session keeps waiting for its first folder read.
pub(super) const MAX_WAITING: usize = 16;

/// Where a refresh's answer goes.
pub(super) type Reply = oneshot::Sender<Option<PullRequestView>>;

/// A refresh that came before its session's first folder read.
#[derive(Debug)]
pub(super) struct Waiting {
    /// A person asked (the look goes first).
    pub(super) person: bool,
    pub(super) reply: Reply,
}

/// What a running job is for, so one that fails is still accounted for.
#[derive(Debug, Clone)]
pub(super) enum Job {
    Folder(SessionId),
    Look(Key),
    Feed(SessionId),
}

/// A session the watcher holds.
#[derive(Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent facts of one held session, each read on its own; a state enum would \
              have to name every combination"
)]
pub(super) struct Session {
    pub(super) handle: SessionHandle,
    pub(super) folder: PathBuf,
    pub(super) project: ProjectId,
    /// A person started it.
    pub(super) person: bool,
    /// It has a queue (structured); a terminal session takes no feedback.
    pub(super) structured: bool,
    /// Its feedback switch: `None` until the store's copy is read with its
    /// first folder read, or a person sets it. A load that comes back
    /// after a person's switch never undoes it.
    pub(super) feedback: Option<bool>,
    /// The signatures sent to it, once loaded.
    pub(super) sent: Option<HashSet<String>>,
    /// The watch it is on.
    pub(super) key: Option<Key>,
    /// Its folder's commit, as last read.
    pub(super) head: Option<CommitId>,
    /// Why it is on no watch.
    pub(super) access: ForgeAccess,
    pub(super) remembered: Option<Remembered>,
    pub(super) due: Instant,
    pub(super) busy: bool,
    /// When it last asked for a refresh (at most one per 10 s).
    pub(super) refreshed: Option<Instant>,
    /// A feedback job runs for it.
    pub(super) feeding: bool,
    /// Its folder has been read once (see the module's second Decision
    /// Log).
    pub(super) read_once: bool,
    /// Refreshes waiting for that first read.
    pub(super) waiting: Vec<Waiting>,
}

/// One repository's branch, and its pull request.
#[derive(Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent facts of one watch, each read on its own; a state enum would have to \
              name every combination"
)]
pub(super) struct Watch {
    pub(super) sessions: BTreeSet<SessionId>,
    /// One of its sessions is a person's.
    pub(super) person: bool,
    /// Who the node is signed in as there.
    pub(super) login: Option<ForgeLogin>,
    /// Why the last look failed, until one succeeds.
    pub(super) error: Option<ForgeAccess>,
    pub(super) listed: Vec<Found>,
    /// The pull request last read and its head then.
    pub(super) known: Option<(PullNumber, CommitId)>,
    pub(super) read: Option<Arc<Read>>,
    /// The branch has been looked at once: its pull requests are seen
    /// (`PullRequests::Seen`), none or one.
    pub(super) seen: bool,
    pub(super) due: Instant,
    pub(super) busy: bool,
    /// The next look is a person's (a refresh).
    pub(super) urgent: bool,
    pub(super) last_find: Option<Instant>,
    pub(super) last_full: Option<Instant>,
    pub(super) changed_at: Instant,
    pub(super) failures: u32,
    /// Refreshes waiting for the next look.
    pub(super) replies: Vec<(SessionId, Reply)>,
}

/// The watcher's state.
#[derive(Debug)]
pub(crate) struct Watcher {
    pub(super) hosts: Arc<Hosts>,
    pub(super) store: Store,
    pub(super) workspaces: Arc<Workspaces>,
    commands: mpsc::Receiver<Command>,
    watched: Arc<AtomicU32>,
    pub(super) sessions: HashMap<SessionId, Session>,
    pub(super) watches: HashMap<Key, Watch>,
    pub(super) jobs: JoinSet<Done>,
    /// What each running job is for, by its task id.
    pub(super) running: HashMap<tokio::task::Id, Job>,
    pub(super) folders: usize,
    pub(super) polls: usize,
    pub(super) writers: Arc<Writers>,
    pub(super) random: RandomState,
}

impl Watcher {
    pub(super) fn new(
        hosts: Arc<Hosts>,
        store: Store,
        workspaces: Arc<Workspaces>,
        commands: mpsc::Receiver<Command>,
        watched: Arc<AtomicU32>,
    ) -> Self {
        Self {
            hosts,
            store,
            workspaces,
            commands,
            watched,
            sessions: HashMap::new(),
            watches: HashMap::new(),
            jobs: JoinSet::new(),
            running: HashMap::new(),
            folders: 0,
            polls: 0,
            writers: Arc::new(Writers::default()),
            random: RandomState::new(),
        }
    }

    /// Watch until `stop`; the jobs still running are dropped with it.
    pub(super) async fn run(mut self, stop: CancellationToken) {
        loop {
            self.start_due(Instant::now());
            let next = self.next_due();
            tokio::select! {
                () = stop.cancelled() => break,
                command = self.commands.recv() => match command {
                    Some(command) => self.command(command),
                    None => break,
                },
                Some(done) = self.jobs.join_next_with_id(), if !self.jobs.is_empty() => match done {
                    Ok((id, done)) => {
                        let _job = self.running.remove(&id);
                        self.done(done);
                    }
                    Err(error) => {
                        tracing::warn!(%error, "a pull request job failed");
                        if let Some(job) = self.running.remove(&error.id()) {
                            self.failed(job);
                        }
                    }
                },
                () = tokio::time::sleep_until(next) => {}
            }
            let count = u32::try_from(self.watches.len()).unwrap_or(u32::MAX);
            self.watched.store(count, Ordering::Relaxed);
        }
        self.jobs.abort_all();
    }

    fn command(&mut self, command: Command) {
        match command {
            Command::Started(started) => self.start(*started),
            Command::Forget(session) => self.forget(session),
            Command::Refresh {
                session,
                person,
                reply,
            } => self.refresh(session, person, reply),
            Command::Get { session, reply } => {
                drop(reply.send(self.view(session)));
            }
            Command::List { reply } => drop(reply.send(self.list())),
            Command::Feedback { session, on, reply } => {
                let held = self
                    .sessions
                    .get_mut(&session)
                    .map(|held| held.feedback = Some(on));
                if held.is_some() {
                    self.publish(session);
                    self.feed(session);
                }
                drop(reply.send(self.info(session)));
            }
        }
    }

    fn start(&mut self, started: Started) {
        let Started {
            handle,
            folder,
            project,
            person,
        } = started;
        if handle.has_ended() {
            return;
        }
        let structured = matches!(
            handle.info().spec.mode,
            vessel_proto::SessionMode::Structured { .. }
        );
        let id = handle.id();
        // Started again: refreshes still waiting wait for this start's
        // first read.
        let waiting = self
            .sessions
            .get_mut(&id)
            .map(|held| std::mem::take(&mut held.waiting))
            .unwrap_or_default();
        self.forget(id);
        let _previous = self.sessions.insert(
            id,
            Session {
                handle,
                folder,
                project,
                person,
                structured,
                feedback: None,
                sent: None,
                key: None,
                head: None,
                access: ForgeAccess::Unknown,
                remembered: None,
                due: Instant::now(),
                busy: false,
                refreshed: None,
                feeding: false,
                read_once: false,
                waiting,
            },
        );
    }

    /// Drop `session` at once (archived, deleted, ended); a refresh still
    /// waiting for its first read is answered: not held.
    pub(super) fn forget(&mut self, session: SessionId) {
        self.detach(session);
        if let Some(gone) = self.sessions.remove(&session) {
            for waiting in gone.waiting {
                drop(waiting.reply.send(None));
            }
        }
    }

    pub(super) fn refresh(&mut self, session: SessionId, person: bool, reply: Option<Reply>) {
        let now = Instant::now();
        let Some(held) = self.sessions.get_mut(&session) else {
            if let Some(reply) = reply {
                drop(reply.send(None));
            }
            return;
        };
        if !held.read_once {
            // Its first read is still to come: read it as soon as a slot is
            // free, and answer with what that read finds (module's second
            // Decision Log). Nothing is stamped: no look happened.
            held.due = held.due.min(now);
            match reply {
                Some(reply) if held.waiting.len() < MAX_WAITING => {
                    held.waiting.push(Waiting { person, reply });
                }
                Some(reply) => drop(reply.send(self.view(session))),
                None => {}
            }
            return;
        }
        let recent = reply.is_some()
            && held
                .refreshed
                .is_some_and(|at| now.saturating_duration_since(at) < view::REFRESH_EVERY);
        if !recent {
            held.refreshed = Some(now);
            held.due = now;
        }
        let watch = held.key.clone().and_then(|key| self.watches.get_mut(&key));
        match (watch, reply) {
            (Some(watch), reply) if !recent => {
                watch.due = now;
                watch.urgent |= person;
                if let Some(reply) = reply {
                    watch.replies.push((session, reply));
                }
            }
            (_, Some(reply)) => drop(reply.send(self.view(session))),
            (_, None) => {}
        }
    }

    /// `session`'s folder was read for the first time: the refreshes that
    /// waited for it join its watch's next look, made now; on no watch they
    /// get the view, which says why.
    pub(super) fn first_read(&mut self, session: SessionId) {
        let now = Instant::now();
        let Some(held) = self.sessions.get_mut(&session) else {
            return;
        };
        held.read_once = true;
        let waiting = std::mem::take(&mut held.waiting);
        if waiting.is_empty() {
            return;
        }
        held.refreshed = Some(now);
        let person = waiting.iter().any(|waiting| waiting.person);
        match held.key.clone().and_then(|key| self.watches.get_mut(&key)) {
            Some(watch) => {
                watch.due = now;
                watch.urgent |= person;
                watch
                    .replies
                    .extend(waiting.into_iter().map(|waiting| (session, waiting.reply)));
            }
            None => {
                for waiting in waiting {
                    drop(waiting.reply.send(self.view(session)));
                }
            }
        }
    }

    /// Give `session` its view now.
    pub(super) fn publish(&self, session: SessionId) {
        if let Some(held) = self.sessions.get(&session) {
            held.handle
                .set_pull_request(Some(view::forge_view(held, self.watch_of(held))));
        }
    }

    pub(super) fn watch_of(&self, held: &Session) -> Option<&Watch> {
        held.key.as_ref().and_then(|key| self.watches.get(key))
    }

    /// What `pr.get` answers for `session`, when held.
    pub(super) fn view(&self, session: SessionId) -> Option<PullRequestView> {
        let held = self.sessions.get(&session)?;
        Some(view::pull_request(held, self.watch_of(held)))
    }

    /// What `SessionInfo.pull_request` holds for `session`, when held.
    fn info(&self, session: SessionId) -> Option<SessionPullRequest> {
        let held = self.sessions.get(&session)?;
        Some(view::info(held, self.watch_of(held)))
    }

    /// Every watched session's, at most [`MAX_WATCHES`] sessions' worth.
    fn list(&self) -> PullRequestList {
        let mut entries: Vec<SessionPullEntry> = self
            .sessions
            .iter()
            .filter(|(_, held)| held.key.is_some())
            .map(|(id, held)| SessionPullEntry {
                session: *id,
                pull_request: view::info(held, self.watch_of(held)),
            })
            .collect();
        entries.sort_by_key(|entry| entry.session);
        entries.truncate(MAX_WATCHES);
        PullRequestList { entries }
    }
}

#[cfg(test)]
mod tests;
