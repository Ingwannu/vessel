//! Projects, worktrees, git and files on a machine (`project.*`,
//! `worktree.*`, `git.*`, `file.*`, protocols 10 and 11): what the
//! workspace's screens ask ([`Ask`]) and what the node answered
//! ([`Answer`]), one typed pair per call.
//!
//! [Decision Log]
//! - Purpose: Settings > Projects, the approval, the code panel and the
//!   worktree start all call the node; the screens must not block on it
//!   and must tell a refusal (typed, from the node) from a machine gone.
//! - Constraint: the link is one task owning the connections (the lead's
//!   `State`); a call that waits (a clone followed for up to 30 s at a
//!   time, setup's output, a worktree start that waits on a person) must
//!   not hold that task, or every other screen would wait behind it.
//! - Alternatives: one `Command` and one `Event` variant per call in the
//!   lead's enums (forty arms in a file every track edits); the generic
//!   `call_any` with the wire's `Reply` (one variant for everything, and a
//!   screen matching replies it did not ask for).
//! - Choice: one `Command::Workspace` and one `Event::Workspace` in the
//!   lead's enums; here, an [`Ask`] per call holding the wire's own request
//!   and an [`Answer`] per call echoing what was asked with a typed result.
//!   Each ask runs as a task of its own on the link's runtime, with a clone
//!   of the machine's connection, so a slow call holds nothing else.
//! - Bounds: asks come from a person or from what is drawn (a diff asked
//!   when its rows are in the window, the Changes tab's refresh every five
//!   seconds while shown): their number is what a screen shows. A clone is
//!   followed by one task per project ([`Hub`] keeps the set); a start by
//!   one task per start.
//! - Cost: an answer for a screen no longer shown still arrives; the
//!   screen drops what does not match what it shows (each answer names
//!   its root, path or project).

mod answer;
mod ask;
mod calls;
mod follow;
mod run;
mod start;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use vessel_app_view::workspace::problem::Problem;
use vessel_client::NodeClient;
use vessel_proto::ProjectId;

pub use answer::{Answer, Form, GitWritten};
pub use ask::{Ask, FileAsk, GitAsk, GitWrite, PathChange, ProjectAsk, Save, WorktreeAsk};
pub use start::{StartAnswer, StartAsk, StartEnded, StartKey};

use crate::Event;

/// The workspace's long-lived part of the link: the clones followed, so a
/// project is followed once however often the list is read.
#[derive(Debug, Clone, Default)]
pub(crate) struct Hub {
    following: Arc<Mutex<HashSet<(String, ProjectId)>>>,
}

impl Hub {
    /// Run `ask` on `machine` (its connection `client`), answering on
    /// `events`; returns at once.
    pub(crate) fn ask(
        &self,
        client: NodeClient,
        machine: String,
        ask: Ask,
        events: mpsc::Sender<Event>,
    ) {
        let hub = self.clone();
        drop(tokio::spawn(async move {
            let reply = Reply {
                machine: machine.clone(),
                events,
            };
            run::run(&hub, &client, &reply, ask).await;
        }));
    }

    /// Answer `ask` with `problem` (the machine is not connected).
    pub(crate) fn refuse(machine: String, ask: Ask, problem: Problem, events: mpsc::Sender<Event>) {
        drop(tokio::spawn(async move {
            let reply = Reply { machine, events };
            reply.send(answer::refused(ask, &problem)).await;
        }));
    }
}

/// Where answers go, for one machine.
#[derive(Debug, Clone)]
pub(crate) struct Reply {
    machine: String,
    events: mpsc::Sender<Event>,
}

impl Reply {
    /// Send `answer`; the receiver is gone only when the app is closing.
    pub(crate) async fn send(&self, answer: Answer) {
        let _closed = self
            .events
            .send(Event::Workspace {
                machine: self.machine.clone(),
                answer,
            })
            .await;
    }
}
