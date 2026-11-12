//! Protocol 10's worktree calls, and where a session runs (track B,
//! ADR 0024): routing and the agent rules; the work is
//! `crate::workspace`'s.

use vessel_proto::workspace::{
    CreateWorktree, CreateWorktreeOutcome, DropSnapshot, ListSnapshots, ListWorktrees,
    ReadSetupOutput, RemoveWorktree, RemoveWorktreeOutcome, RunSetup, SessionPlace, SetupOutput,
    SnapshotList, WorktreeInfo, WorktreeList,
};
use vessel_proto::{Ack, Creator, ErrorCode, SessionInfo, SessionSpec, WireError};

use super::projects::wire;
use super::{Connection, NodeService};
use crate::principal::Principal;
use crate::profiles::launch::creator;
use crate::waits::Waiter;
use crate::workspace::Placed;

impl NodeService {
    /// `worktree.list`.
    #[expect(
        clippy::unused_async,
        reason = "every call has the same async shape, so the guard's rows are uniform"
    )]
    pub(crate) async fn list_worktrees(
        &self,
        _: &Connection,
        request: ListWorktrees,
    ) -> Result<WorktreeList, WireError> {
        let live = self.sessions.live_folders();
        Ok(self.workspaces.list_worktrees(request.project, &live))
    }

    /// `worktree.create`: the caller is its creator; an agent's counts
    /// against its bound of worktrees.
    pub(crate) async fn create_worktree(
        &self,
        connection: &Connection,
        request: CreateWorktree,
    ) -> Result<CreateWorktreeOutcome, WireError> {
        let creator = creator(connection.principal());
        self.workspaces
            .create_worktree(creator, request, None)
            .await
            .map_err(|error| wire(&error))
    }

    /// `worktree.output`: a long poll as `project.get`'s.
    pub(crate) async fn setup_output(
        &self,
        connection: &Connection,
        mut request: ReadSetupOutput,
    ) -> Result<SetupOutput, WireError> {
        if matches!(connection.principal(), Principal::Delegated { .. }) {
            request.wait = None;
        }
        let here = self.links.node();
        let shares = Waiter::of(connection.principal(), here).shares(here);
        let _slot = self.wait_slot(connection, shares, &mut request.wait);
        self.workspaces
            .read_output(request.worktree, request.from, request.wait)
            .await
            .map_err(|error| wire(&error))
    }

    /// `worktree.setup`.
    pub(crate) async fn run_setup(
        &self,
        _: &Connection,
        request: RunSetup,
    ) -> Result<WorktreeInfo, WireError> {
        self.workspaces
            .run_setup(request.worktree)
            .await
            .map_err(|error| wire(&error))
    }

    /// `worktree.remove`.
    pub(crate) async fn remove_worktree(
        &self,
        _: &Connection,
        request: RemoveWorktree,
    ) -> Result<RemoveWorktreeOutcome, WireError> {
        let live = self.sessions.live_folders();
        self.workspaces
            .remove_worktree(request.worktree, request.if_lost, &live)
            .await
            .map_err(|error| wire(&error))
    }

    /// `worktree.snapshots`.
    pub(crate) async fn list_snapshots(
        &self,
        _: &Connection,
        request: ListSnapshots,
    ) -> Result<SnapshotList, WireError> {
        self.workspaces
            .list_snapshots(request.project)
            .await
            .map_err(|error| wire(&error))
    }

    /// `worktree.drop_snapshot`.
    pub(crate) async fn drop_snapshot(
        &self,
        _: &Connection,
        request: DropSnapshot,
    ) -> Result<Ack, WireError> {
        self.workspaces
            .drop_snapshot(request.project, request.worktree)
            .await
            .map(|()| Ack)
            .map_err(|error| wire(&error))
    }

    /// Where `session.create`'s `place` runs: the folder (filled into
    /// `cwd`, so an older client still sees where the session runs) and
    /// the notices its log starts with. An agent may start in any project
    /// and in a worktree it created, or one created for a session it may
    /// operate (ADR 0013's ownership).
    pub(super) async fn place(
        &self,
        connection: &Connection,
        spec: &mut SessionSpec,
    ) -> Result<Vec<String>, WireError> {
        let Some(place) = spec.place.as_deref() else {
            return Ok(Vec::new());
        };
        if spec.cwd.is_some() {
            return Err(WireError::new(
                ErrorCode::InvalidRequest,
                "a session with a place gets its folder from the place; leave `cwd` out",
            ));
        }
        if let SessionPlace::Worktree { worktree } = place
            && let Some(agent) = connection.principal().agent()
        {
            let (made_by, made_for) = self
                .workspaces
                .owner_of(*worktree)
                .map_err(|error| wire(&error))?;
            let own = made_by
                == Creator::Agent {
                    node: agent.0,
                    session: agent.1,
                };
            if !own {
                let Some(session) = made_for else {
                    return Err(WireError::new(
                        ErrorCode::Forbidden,
                        "an agent may start a session only in a worktree it created, or one \
                         created for a session it may operate",
                    ));
                };
                self.check_owner(connection.principal(), session).await?;
            }
        }
        let Placed { cwd, notices } = self
            .workspaces
            .place(place)
            .await
            .map_err(|error| wire(&error))?;
        spec.cwd = Some(cwd);
        Ok(notices)
    }

    /// `info` with its project, worktree and branch filled from its spec's
    /// place, as the node knows them now.
    pub(super) fn with_workspace(&self, mut info: SessionInfo) -> SessionInfo {
        info.workspace = self.workspaces.workspace_of(&info.spec).map(Box::new);
        info
    }
}
