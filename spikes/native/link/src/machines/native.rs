//! Commands stay ordered with link's other state changes; only file I/O leaves
//! its actor, so a rename cannot race a refresh of the same connection map.
use super::{Answer, Ask, Machine, edits};
use crate::{Event, State, nodes};
use std::collections::HashMap;
use vessel_app_view::machine_book::{Edit, Failure};
use vessel_client::book::NodeBook;

impl State {
    pub(crate) async fn machine_book(&mut self, ask: Ask) {
        match ask {
            Ask::Read => self.machine_snapshot().await,
            Ask::Retry => {
                self.refresh().await;
                self.machine_snapshot().await;
            }
            Ask::Edit(edit) => {
                let result = self.machine_edit(edit.clone()).await;
                self.emit(Event::Machines(Answer::Edited { edit, result }))
                    .await;
                self.refresh().await;
                self.machine_snapshot().await;
            }
        }
    }

    async fn machine_snapshot(&self) {
        let result = self.read_machines().await;
        self.emit(Event::Machines(Answer::Read(result))).await;
    }

    async fn read_machines(&self) -> Result<Vec<Machine>, Failure> {
        let dir = nodes::config_dir().map_err(|_error| Failure::NoConfiguration)?;
        let known: HashMap<_, _> = self
            .clients
            .iter()
            .map(|(name, connected)| {
                (
                    name.clone(),
                    (connected.client.node().clone(), connected.client.protocol()),
                )
            })
            .collect();
        tokio::task::spawn_blocking(move || {
            edits::read(&NodeBook::path_in(&dir), |name| known.get(name).cloned())
                .map_err(|error| error.failure())
        })
        .await
        .map_err(|_error| Failure::Worker)?
    }

    async fn machine_edit(&mut self, edit: Edit) -> Result<(), Failure> {
        let dir = nodes::config_dir().map_err(|_error| Failure::NoConfiguration)?;
        let writing = edit.clone();
        tokio::task::spawn_blocking(move || {
            edits::write(&NodeBook::path_in(&dir), &writing).map_err(|error| error.failure())
        })
        .await
        .map_err(|_error| Failure::Worker)??;
        match edit {
            Edit::Rename { from, to } => {
                if self.follow_machine.as_deref() == Some(from.as_str()) {
                    self.follow_machine = Some(to.as_str().to_owned());
                }
                if let Some(connected) = self.clients.remove(from.as_str()) {
                    let _old = self.clients.insert(to.as_str().to_owned(), connected);
                }
            }
            Edit::Forget(name) => {
                let _removed = self.clients.remove(name.as_str());
                // A watched conversation owns another client clone. Forgetting
                // must stop that stream too; the node's session keeps running.
                if self.follow_machine.as_deref() == Some(name.as_str()) {
                    if let Some(follow) = self.follow.take() {
                        follow.abort();
                    }
                    self.follow_machine = None;
                }
            }
            Edit::MakeDefault(_) => {}
        }
        Ok(())
    }
}
