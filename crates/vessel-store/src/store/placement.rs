//! Async runtime registration of project roots (ADR 0106); SQL stays on the
//! store's writer and read threads.

use chrono::{DateTime, Utc};
use vessel_proto::{ProjectId, WorkspaceId};

use super::Store;
use crate::{Placement, StoreError, registry, writer::Write};

impl Store {
    /// The project root's registry workspace: the oldest Active, foreground
    /// `Local` one at `placement`, or a new one with id `fresh` created at
    /// `at`. Lookup and insert share one savepoint, so concurrent callers at
    /// one root get one workspace. Registry observers wake when it is new.
    ///
    /// # Errors
    /// `RegistryFull` with no live slot left, or a database failure.
    pub async fn register_workspace(
        &self,
        placement: Placement,
        fresh: WorkspaceId,
        at: DateTime<Utc>,
    ) -> Result<WorkspaceId, StoreError> {
        self.write(|done| Write::Placement {
            register: Box::new(registry::Register {
                placement,
                fresh,
                at,
            }),
            done,
        })
        .await
    }

    /// Node start's one-shot bootstrap: projects with placed, non-archived
    /// sessions and no registry record in any lifecycle, each with its
    /// earliest such session's creation time.
    ///
    /// # Errors
    /// A database failure or a corrupt stored id.
    pub async fn unregistered_placed_projects(
        &self,
    ) -> Result<Vec<(ProjectId, DateTime<Utc>)>, StoreError> {
        self.read(|reader| reader.with(registry::unregistered_placed))
            .await
    }
}
