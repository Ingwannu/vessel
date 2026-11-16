//! Each label-capable host's catalog and registry, followed while it is
//! connected (Paseo's `workspace-labels/internal/host-replica.ts`: a host's
//! labels stay current without a reload).
//!
//! [Decision Log]
//! - Purpose: the manager, the picker and the session rows draw the host's
//!   real labels; another device's edit or assignment shows within moments.
//! - Constraint: the node's catalog read and registry read are finite
//!   calls that wait for a change from a cursor or revision (or answer at
//!   their heartbeat); an answer must apply only to the admission that
//!   asked; a gap in the catalog's sequence needs a fresh snapshot.
//! - Choice: per host, one catalog read and one registry read out at a
//!   time: the first at once, every later one waiting from where the last
//!   answer left off. The catalog goes through the view's `Replica`
//!   (contiguous changes, a snapshot after a gap); the registry through
//!   `Assignments`. Each answer is followed by the file watch's quiet
//!   pause, a failed read by a longer one; an older peer, a closed or
//!   replaced admission stops the host's reads until it binds again.
//! - A sequence gap in the catalog reads a snapshot at once, the host
//!   staying Loading. A read that comes back at once with nothing new
//!   (the node had no wait share left, or the caller is delegated) backs
//!   off, doubling the pause up to 30 s, until something changes; an
//!   answer that changes nothing redraws nothing.
//! - Cost: two waiting calls per connected host (each holds one of the
//!   node's wait shares); answers to a host that went away are dropped.

use std::time::{Duration, Instant};

use vessel_app_kit::ui::Context;
use vessel_app_link::labels::{Answer, Command, Event, Request};
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::labels::{
    Assignments, EditAccess, Failure, HostKey, HostState, Label, Replica, Ticket,
};
use vessel_app_view::workspace::pace::QUIET;
use vessel_proto::workspace_labels::{CatalogMode, CatalogRead};

use crate::shell::Shell;

/// A session row's workspace labels: its host's admission, its registry
/// workspace, its labels in order with their host colours, and whether
/// the host takes changes now.
pub(crate) struct RowLabels<'a> {
    pub(crate) connection: &'a ConnectionContext,
    pub(crate) workspace: vessel_proto::WorkspaceId,
    pub(crate) labels: Vec<Label>,
    pub(crate) writable: bool,
}

/// The pause after a failed read, before the host is read again.
const AFTER_FAILURE: Duration = Duration::from_secs(10);
/// An answer sooner than this did not wait (no wait share left).
const QUICK: Duration = Duration::from_secs(2);
/// The longest pause after answers that came back at once with nothing.
const LONGEST: Duration = Duration::from_secs(30);

/// Which of a host's two reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Read {
    Catalog,
    Registry,
}

/// Where one read is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Lane {
    /// Nothing out: ask at the next chance.
    #[default]
    Idle,
    /// This request is out since `at`.
    Out { ticket: Ticket, at: Instant },
    /// Answered; the pause numbered here runs before the next ask.
    Resting(u64),
    /// Not read again until the host binds again.
    Stopped,
}

/// One host's followed state.
#[derive(Debug)]
pub(super) struct Follow {
    pub(super) key: HostKey,
    connection: ConnectionContext,
    replica: Replica,
    pub(super) registry: Assignments,
    catalog: Lane,
    registry_lane: Lane,
    /// The number of the latest pause.
    rests: u64,
    /// Answers in a row that came back at once with nothing new, per read.
    idle: [u32; 2],
    /// The admission answered that it has no label calls: a bind of the
    /// same admission keeps the host Unsupported (it would say Loading,
    /// with no read left to settle it).
    unsupported: bool,
}

impl Follow {
    fn lane(&mut self, read: Read) -> &mut Lane {
        match read {
            Read::Catalog => &mut self.catalog,
            Read::Registry => &mut self.registry_lane,
        }
    }

    fn idle(&mut self, read: Read) -> &mut u32 {
        let [catalog, registry] = &mut self.idle;
        match read {
            Read::Catalog => catalog,
            Read::Registry => registry,
        }
    }
}

impl Shell {
    /// After a bind: follow every connected, capable host; forget the rest.
    pub(super) fn labels_follow(&mut self, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.labels;
        let capable: Vec<(HostKey, ConnectionContext)> = state
            .hosts
            .iter()
            .filter(|host| !matches!(host.state, HostState::Unsupported | HostState::Offline))
            .filter_map(|host| Some((host.key, host.connection.clone()?)))
            .collect();
        state
            .follows
            .retain(|follow| capable.iter().any(|(key, _)| *key == follow.key));
        for (key, connection) in &capable {
            let (key, connection) = (*key, connection.clone());
            match state.follows.iter_mut().find(|follow| follow.key == key) {
                // A new admission: the cache stays (the cursor catches up),
                // the reads start again on it.
                Some(follow) if follow.connection != connection => {
                    follow.connection = connection;
                    follow.catalog = Lane::Idle;
                    follow.registry_lane = Lane::Idle;
                    follow.unsupported = false;
                }
                Some(_same) => {}
                None => state.follows.push(Follow {
                    key,
                    connection,
                    replica: Replica::default(),
                    registry: Assignments::default(),
                    catalog: Lane::Idle,
                    registry_lane: Lane::Idle,
                    rests: 0,
                    idle: [0, 0],
                    unsupported: false,
                }),
            }
        }
        for follow in state.follows.iter().filter(|follow| follow.unsupported) {
            if let Some(host) = state.hosts.iter_mut().find(|host| host.key == follow.key) {
                host.state = HostState::Unsupported;
            }
        }
        for (key, _connection) in capable {
            self.labels_read(key, Read::Catalog);
            self.labels_read(key, Read::Registry);
        }
        cx.notify();
    }

    /// Ask `read` of `key` if nothing is out or resting.
    fn labels_read(&mut self, key: HostKey, read: Read) {
        let state = &mut self.areas.labels;
        let Some(follow) = state.follows.iter_mut().find(|follow| follow.key == key) else {
            return;
        };
        if *follow.lane(read) != Lane::Idle {
            return;
        }
        let Ok(ticket) = state.tickets.issue() else {
            *follow.lane(read) = Lane::Stopped;
            return;
        };
        let ask = match read {
            Read::Catalog => {
                let cursor = follow.replica.cursor();
                Command::List {
                    cursor,
                    read: if cursor.is_some() {
                        CatalogRead::Observe
                    } else {
                        CatalogRead::Now
                    },
                }
            }
            Read::Registry => Command::Registry {
                changed_from: follow.registry.revision(),
            },
        };
        *follow.lane(read) = Lane::Out {
            ticket,
            at: Instant::now(),
        };
        let context = follow.connection.clone();
        self.command(vessel_app_link::Command::Labels(Request {
            context,
            ticket,
            ask,
        }));
    }

    /// A followed read's answer, or an assignment's committed labels.
    /// Whether the event was a followed read's (and so taken here).
    pub(super) fn labels_followed(&mut self, event: &Event, cx: &mut Context<'_, Self>) -> bool {
        if let Answer::Assigned {
            workspace,
            result: Ok(labels),
            ..
        } = &event.answer
        {
            if let Some(follow) = self.follow_of(&event.context) {
                follow.registry.assigned(*workspace, labels.clone());
            }
            self.labels_picker_refresh(cx);
            return false;
        }
        let read = match &event.answer {
            Answer::Listed(_) => Read::Catalog,
            Answer::Registry(_) => Read::Registry,
            Answer::Assigned { .. }
            | Answer::Updated(_)
            | Answer::Inspected(_)
            | Answer::Deleted(_) => return false,
        };
        let Some(follow) = self.follow_of(&event.context) else {
            return true;
        };
        let Lane::Out { ticket, at } = *follow.lane(read) else {
            return true;
        };
        if ticket != event.ticket {
            return true;
        }
        let key = follow.key;
        let quick = at.elapsed() < QUICK;
        let outcome = settle(follow, read, &event.answer, quick);
        follow.unsupported |= outcome.unsupported;
        follow.rests = follow.rests.wrapping_add(1);
        let rest = follow.rests;
        *follow.lane(read) = match outcome.pause {
            Some(_) => Lane::Resting(rest),
            None => Lane::Stopped,
        };
        let catalog = (read == Read::Catalog)
            .then(|| (follow.replica.catalog().clone(), follow.replica.state()));
        let mut redraw = outcome.changed;
        if let Some(host) = self
            .areas
            .labels
            .hosts
            .iter_mut()
            .find(|host| host.key == key)
        {
            if outcome.unsupported {
                redraw |= host.state != HostState::Unsupported;
                host.state = HostState::Unsupported;
            } else if let Some((catalog, state)) = catalog
                && outcome.pause.is_some()
                && (host.catalog != catalog || host.state != state)
            {
                host.catalog = catalog;
                host.state = state;
                redraw = true;
            }
        }
        if let Some(wait) = outcome.pause {
            rest_then_read(key, read, rest, wait, cx);
        }
        if redraw {
            if read == Read::Registry {
                self.labels_picker_refresh(cx);
            }
            self.settle_labels(cx);
            cx.notify();
        }
        true
    }

    fn follow_of(&mut self, connection: &ConnectionContext) -> Option<&mut Follow> {
        self.areas
            .labels
            .follows
            .iter_mut()
            .find(|follow| &follow.connection == connection)
    }

    /// The Labels item of `session`'s row menu: the picker for its
    /// workspace, with the registry's labels.
    pub(crate) fn labels_row_open(
        &mut self,
        machine: &str,
        session: vessel_proto::SessionId,
        window: &mut vessel_app_kit::ui::Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(row) = self.areas.labels.row(machine, session) else {
            return;
        };
        let key = vessel_app_view::layout::LayoutKey::Workspace {
            node: row.connection.node(),
            workspace: row.workspace,
        };
        let Ok(target) =
            vessel_app_view::labels::WorkspaceTarget::new(row.connection.clone(), &key)
        else {
            return;
        };
        let assigned = row.labels.iter().map(|label| label.name.clone()).collect();
        let access = if row.writable {
            EditAccess::Editable
        } else {
            EditAccess::ReadOnly
        };
        self.labels_picker_open(target, assigned, access, window, cx);
    }

    /// The open picker's assigned labels and access, from its host's
    /// followed registry.
    fn labels_picker_refresh(&mut self, cx: &mut Context<'_, Self>) {
        let Some((target, assigned, access)) = self.areas.labels.picker_view() else {
            return;
        };
        self.labels_picker_sync(&target, assigned, access, cx);
    }
}

impl super::Labels {
    /// The open picker's target with its registry labels and access now.
    fn picker_view(
        &self,
    ) -> Option<(
        vessel_app_view::labels::WorkspaceTarget,
        Vec<vessel_app_view::labels::Name>,
        EditAccess,
    )> {
        let target = self.picker_target()?;
        let follow = self
            .follows
            .iter()
            .find(|follow| &follow.connection == target.connection())?;
        let assigned = follow.registry.labels(target.workspace()).to_vec();
        let writable = self
            .hosts
            .iter()
            .find(|host| host.key == follow.key)
            .is_some_and(|host| host.state.writable().is_ok());
        let access = if writable {
            EditAccess::Editable
        } else {
            EditAccess::ReadOnly
        };
        Some((target, assigned, access))
    }

    /// A session row's workspace and labels, on `machine`'s host: none
    /// while the host is not followed or the session has no workspace.
    pub(crate) fn row(
        &self,
        machine: &str,
        session: vessel_proto::SessionId,
    ) -> Option<RowLabels<'_>> {
        let host = self
            .hosts
            .iter()
            .find(|host| host.route.as_str() == machine)?;
        let connection = host.connection.as_ref()?;
        let follow = self
            .follows
            .iter()
            .find(|follow| &follow.connection == connection)?;
        let workspace = follow.registry.workspace(session)?;
        let labels = follow
            .registry
            .labels(workspace)
            .iter()
            .map(|name| {
                // A name the catalog does not hold yet (its read is behind
                // the registry's) keeps the create colour until it arrives.
                host.catalog.get(name).cloned().unwrap_or_else(|| Label {
                    name: name.clone(),
                    color: vessel_app_view::labels::Color::default(),
                })
            })
            .collect();
        Some(RowLabels {
            connection,
            workspace,
            labels,
            writable: host.state.writable().is_ok(),
        })
    }
}

/// What an answer did to a host's followed state.
struct Outcome {
    /// The pause before the next read; none stops the reads.
    pause: Option<Duration>,
    /// Whether the catalog or the registry changed (something to redraw).
    changed: bool,
    /// Whether the admission has no label calls.
    unsupported: bool,
}

/// Apply `answer` to `follow`'s `read`; `quick`: it came back without
/// waiting.
fn settle(follow: &mut Follow, read: Read, answer: &Answer, quick: bool) -> Outcome {
    let (pause, changed) = match answer {
        Answer::Listed(Ok(list)) => {
            let before = (follow.replica.cursor(), follow.replica.state());
            let gap = follow.replica.listed(list).is_err();
            if gap && matches!(list.data, CatalogMode::Changes { .. }) {
                // Read the whole catalog again at once, still Loading.
                follow.replica.reconnecting();
                (Some(Duration::ZERO), false)
            } else if gap {
                (Some(AFTER_FAILURE), true)
            } else {
                let changed = before != (follow.replica.cursor(), follow.replica.state());
                (Some(pace(follow.idle(read), changed, quick)), changed)
            }
        }
        Answer::Registry(Ok(snapshot)) => {
            // By value, not revision: a snapshot of the same revision can
            // end an assignment kept over it, which the rows must show.
            let before = follow.registry.clone();
            follow.registry.refreshed(snapshot);
            let changed = follow.registry != before;
            (Some(pace(follow.idle(read), changed, quick)), changed)
        }
        Answer::Listed(Err(failure)) | Answer::Registry(Err(failure)) => match failure {
            Failure::Offline | Failure::Changed | Failure::Unsupported => (None, false),
            Failure::Pending
            | Failure::NameTaken
            | Failure::NotFound
            | Failure::NoChange
            | Failure::Load
            | Failure::Update
            | Failure::TicketExhausted => {
                if read == Read::Catalog {
                    follow.replica.failed(Failure::Load);
                }
                (Some(AFTER_FAILURE), true)
            }
        },
        Answer::Assigned { .. } | Answer::Updated(_) | Answer::Inspected(_) | Answer::Deleted(_) => {
            (None, false)
        }
    };
    Outcome {
        pause,
        changed,
        unsupported: matches!(
            answer,
            Answer::Listed(Err(Failure::Unsupported)) | Answer::Registry(Err(Failure::Unsupported))
        ),
    }
}

/// The pause after an answer: the quiet one, doubled for each answer in a
/// row that came back at once with nothing new (up to `LONGEST`).
fn pace(idle: &mut u32, changed: bool, quick: bool) -> Duration {
    if changed || !quick {
        *idle = 0;
        return QUIET;
    }
    *idle = idle.saturating_add(1).min(5);
    QUIET.saturating_mul(2_u32.saturating_pow(*idle)).min(LONGEST)
}

/// After `wait`, `read` of `key` may ask again (if still resting from the
/// same answer).
fn rest_then_read(
    key: HostKey,
    read: Read,
    rest: u64,
    wait: Duration,
    cx: &mut Context<'_, Shell>,
) {
    cx.spawn(async move |this, cx| {
        cx.background_executor().timer(wait).await;
        let _gone = this.update(cx, |shell, _cx| {
            let follows = &mut shell.areas.labels.follows;
            if let Some(follow) = follows.iter_mut().find(|follow| follow.key == key)
                && *follow.lane(read) == Lane::Resting(rest)
            {
                *follow.lane(read) = Lane::Idle;
                shell.labels_read(key, read);
            }
        });
    })
    .detach();
}

#[cfg(test)]
#[path = "follow_tests.rs"]
mod tests;
