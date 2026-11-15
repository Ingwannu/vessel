//! Commands remain ordered with the link actor. The configured book path is
//! fixed at startup, so every operation reads the same file and tests need no
//! process-wide environment changes. File I/O always uses the blocking pool.
use super::{Answer, Ask, Machine, edits};
use crate::{Event, State};
use std::collections::HashMap;
use vessel_app_view::machine_book::{Edit, Failure};
use vessel_client::book::{BookEntry, NodeName};

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
                let succeeded = result.is_ok();
                if let Err(Failure::File(fault)) = &result {
                    self.emit(Event::BookUnreadable(fault.clone())).await;
                }
                self.emit(Event::Machines(Answer::Edited { edit, result }))
                    .await;
                // A failed local write must not touch clients or the active
                // stream, even if an unrelated connection has since dropped.
                if succeeded {
                    self.refresh().await;
                }
                self.machine_snapshot().await;
            }
        }
    }

    async fn machine_snapshot(&self) {
        self.emit(Event::Machines(Answer::Read(self.read_machines().await)))
            .await;
    }

    /// Tick, reload and explicit retry all use the same safe file boundary.
    pub(crate) async fn saved_entries(&self) -> Result<Vec<(NodeName, BookEntry)>, Failure> {
        let path = self.book_path.path()?.to_owned();
        tokio::task::spawn_blocking(move || edits::entries(&path).map_err(|error| error.failure()))
            .await
            .map_err(|_stopped| Failure::Worker)?
    }

    async fn read_machines(&self) -> Result<Vec<Machine>, Failure> {
        let path = self.book_path.path()?.to_owned();
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
            edits::read(&path, |name| known.get(name).cloned()).map_err(|error| error.failure())
        })
        .await
        .map_err(|_stopped| Failure::Worker)?
    }

    async fn machine_edit(&mut self, edit: Edit) -> Result<(), Failure> {
        let path = self.book_path.path()?.to_owned();
        let writing = edit.clone();
        tokio::task::spawn_blocking(move || {
            edits::write(&path, &writing).map_err(|error| error.failure())
        })
        .await
        .map_err(|_stopped| Failure::Worker)??;
        // Track F's follows of a renamed or forgotten machine end here,
        // after the book file was written (the seam agreed with E2).
        self.workspace.book_applied(&edit);
        self.book_connections_applied(edit).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
