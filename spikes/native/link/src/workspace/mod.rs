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

#[cfg(not(target_arch = "wasm32"))]
mod actor;
mod answer;
mod ask;
mod calls;
pub mod command_access;
mod follow;
pub mod layout;
mod pulls;
mod run;
mod scripts;
mod start;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use tokio::task::AbortHandle;
use vessel_app_view::machine_book::Edit;
use vessel_app_view::workspace::problem::Problem;
use vessel_client::NodeClient;
use vessel_proto::ProjectId;

pub use answer::{Answer, Form, GitWritten};
pub use ask::{Ask, FileAsk, GitAsk, GitWrite, PathChange, ProjectAsk, Save, WorktreeAsk};
pub use pulls::{PullAnswer, PullAsk};
pub use scripts::{ScriptAnswer, ScriptAsk};
pub use start::{StartAnswer, StartAsk, StartEnded, StartKey};

use crate::Event;

/// The workspace's long-lived part of the link: the clones followed, so a
/// project is followed once however often the list is read.
#[derive(Debug, Clone, Default)]
pub(crate) struct Hub {
    /// Each clone followed, by machine and project, with its task (each
    /// task holds a clone of the machine's client until it ends).
    following: Arc<Mutex<HashMap<(String, ProjectId), AbortHandle>>>,
}

impl Hub {
    /// The node book was edited (track E2's Settings › Machines, called in
    /// the link's own task right after the book file was written): a
    /// forgotten machine's follows are cancelled, so nothing here keeps its
    /// connection; a renamed machine's too, as each answers under the name
    /// it started with (the app lists the projects again under the new
    /// name, which follows them anew). Nothing is stopped on the machine.
    pub(crate) fn book_applied(&self, edit: &Edit) {
        match edit {
            Edit::Rename { from, .. } => self.stop_following(from.as_str()),
            Edit::Forget(name) => self.stop_following(name.as_str()),
            Edit::MakeDefault(_) => {}
        }
    }

    /// Cancel every follow of `machine`'s.
    fn stop_following(&self, machine: &str) {
        let Ok(mut following) = self.following.lock() else {
            return;
        };
        following.retain(|(on, _), task| {
            let keep = on != machine;
            if !keep {
                task.abort();
            }
            keep
        });
    }

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

#[cfg(test)]
mod book_tests {
    //! The book's edits reach the follows: a forget during a running
    //! follow leaves no task and nothing it held; a rename does the same
    //! for the old name and keeps the other machines' follows.

    use std::sync::Arc;

    use vessel_app_view::machine_book::{Edit, Name};
    use vessel_proto::ProjectId;

    use super::Hub;

    fn project(n: u8) -> ProjectId {
        format!("00000000-0000-4000-8000-0000000000{n:02}")
            .parse()
            .expect("a project id")
    }

    /// A follow that runs until cancelled, holding `held` (as a real one
    /// holds the machine's client).
    fn follow(
        hub: &Hub,
        machine: &str,
        id: ProjectId,
        held: &Arc<()>,
    ) -> tokio::task::JoinHandle<()> {
        let kept = Arc::clone(held);
        let task = tokio::spawn(async move {
            std::future::pending::<()>().await;
            drop(kept);
        });
        let _was = hub
            .following
            .lock()
            .expect("the registry")
            .insert((machine.to_owned(), id), task.abort_handle());
        task
    }

    #[tokio::test]
    async fn a_forget_cancels_the_machine_s_follows_and_drops_what_they_held() {
        let hub = Hub::default();
        let laptop = Arc::new(());
        let server = Arc::new(());
        let first = follow(&hub, "laptop", project(1), &laptop);
        let second = follow(&hub, "laptop", project(2), &laptop);
        let other = follow(&hub, "build-server", project(3), &server);
        hub.book_applied(&Edit::Forget(Name::parse("laptop").expect("a name")));
        assert!(first.await.expect_err("cancelled").is_cancelled());
        assert!(second.await.expect_err("cancelled").is_cancelled());
        assert_eq!(Arc::strong_count(&laptop), 1, "no task holds the client");
        let left: Vec<_> = hub
            .following
            .lock()
            .expect("the registry")
            .keys()
            .cloned()
            .collect();
        assert_eq!(left, vec![("build-server".to_owned(), project(3))]);
        assert_eq!(
            Arc::strong_count(&server),
            2,
            "the other machine's follow runs"
        );
        other.abort();
    }

    #[tokio::test]
    async fn a_rename_cancels_the_old_name_s_follows_and_a_default_changes_nothing() {
        let hub = Hub::default();
        let held = Arc::new(());
        let task = follow(&hub, "laptop", project(1), &held);
        hub.book_applied(&Edit::MakeDefault(Name::parse("laptop").expect("a name")));
        assert_eq!(hub.following.lock().expect("the registry").len(), 1);
        hub.book_applied(&Edit::Rename {
            from: Name::parse("laptop").expect("a name"),
            to: Name::parse("work-laptop").expect("a name"),
        });
        assert!(task.await.expect_err("cancelled").is_cancelled());
        assert_eq!(Arc::strong_count(&held), 1);
        assert!(hub.following.lock().expect("the registry").is_empty());
    }
}
