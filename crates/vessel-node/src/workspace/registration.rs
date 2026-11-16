//! Runtime registration of project roots (ADR 0106): a project root where a
//! session, terminal or script starts becomes its `Local` registry workspace,
//! and node start registers the roots existing sessions were placed in.
//!
//! [Decision Log]
//! - Purpose: give `workspace.registry`, labels, scripts and services the
//!   workspaces Paseo has (`workspace-registry-bootstrap.ts`, and
//!   `findOrCreateWorkspaceForDirectory` when work opens a directory).
//! - Choice: never refuse the caller's start. A registry failure (for example
//!   `RegistryFull`) is logged and the start goes on, as the coordinator ruled
//!   that a live create must never be refused for registry bookkeeping.
//! - Bootstrap: one shot per project. A project that has a registry record in
//!   any lifecycle is skipped, so a workspace archived by removal is not
//!   recreated at the next start. Missing roots are skipped, as Paseo skips an
//!   agent whose directory is gone.
//! - Worktrees are not registered here: `WorkspaceDescriptor::root()` maps
//!   `Local` to the project root, and a worktree needs its own decision.

use chrono::{DateTime, Utc};
use vessel_proto::{ProjectId, WorkspaceId};
use vessel_store::Placement;

use super::Workspaces;

impl Workspaces {
    /// Register `project`'s root as its registry workspace (find or create).
    /// Logs and returns on failure; the caller's start is never refused.
    pub(crate) async fn register_root(&self, project: ProjectId) {
        self.register_root_at(project, Utc::now()).await;
    }

    async fn register_root_at(&self, project: ProjectId, at: DateTime<Utc>) {
        let Ok(path) = self.project_root(project) else {
            return;
        };
        let placement = Placement { project, path };
        match self
            .store
            .register_workspace(placement, WorkspaceId::random(), at)
            .await
        {
            Ok(Some(_workspace)) => {}
            // Removed between resolving the root and the write: nothing to register.
            Ok(None) => tracing::debug!(%project, "the project was removed before registration"),
            Err(error) => {
                tracing::warn!(%error, %project, "could not register the project's workspace");
            }
        }
    }

    /// Node start: register the root of every project whose sessions were
    /// placed in it and that has no registry record yet, created at its
    /// earliest such session's time. Never fails the start.
    pub(super) async fn register_placed_projects(&self) {
        let found = match self.store.unregistered_placed_projects().await {
            Ok(found) => found,
            Err(error) => {
                tracing::warn!(%error, "could not read the projects to register");
                return;
            }
        };
        for (project, earliest) in found {
            let Ok(root) = self.project_root(project) else {
                continue;
            };
            if !tokio::fs::metadata(&root)
                .await
                .is_ok_and(|metadata| metadata.is_dir())
            {
                continue;
            }
            self.register_root_at(project, earliest).await;
        }
    }
}
