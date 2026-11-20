//! Protocol 11's pull request calls (ADR 0029), routed to the node's
//! `forge` module, which decides; and the hooks that tell it when a
//! session starts, is archived or is deleted. The guard has checked the
//! scope, the agents' rule and, for `pr.open`, `pr.merge` and
//! `pr.feedback`, the caller's ownership of the session; `pr.get`'s
//! refresh is held to ownership here.

use vessel_proto::forge::{
    ForgeAccess, ForgeStatus, GetForgeStatus, GetPullRequest, ListPullRequests, MergeOutcome,
    MergePullRequest, OpenOutcome, OpenPullRequest, PullRequestList, PullRequestView,
    RepositoryList, SearchRepositories, SessionPullRequest, SetFeedback,
};
use vessel_proto::{ErrorCode, SessionId, SessionMode, WireError};
use vessel_session::SessionHandle;

use super::{Connection, NodeService, wire};
use crate::forge::{Command, Started};
use crate::principal::Principal;

impl NodeService {
    /// `forge.status`.
    pub(crate) async fn forge_status(
        &self,
        _: &Connection,
        _: GetForgeStatus,
    ) -> Result<ForgeStatus, WireError> {
        Ok(self.forges.status().await)
    }

    /// `forge.repos` (ADR 0088): the guard refused agents; a person's
    /// Add project sheet lists and searches the account's repositories.
    pub(crate) async fn search_repositories(
        &self,
        _: &Connection,
        request: SearchRepositories,
    ) -> Result<RepositoryList, WireError> {
        Ok(self
            .forges
            .repositories(&request.query, request.limit)
            .await)
    }

    /// `pr.list`.
    pub(crate) async fn list_pull_requests(
        &self,
        _: &Connection,
        _: ListPullRequests,
    ) -> Result<PullRequestList, WireError> {
        self.forges.ask(|reply| Command::List { reply }).await
    }

    /// `pr.get`: the session's pull request as the watcher holds it, or
    /// looked at now (`refresh`, on a session the caller may operate).
    pub(crate) async fn get_pull_request(
        &self,
        connection: &Connection,
        request: GetPullRequest,
    ) -> Result<PullRequestView, WireError> {
        let GetPullRequest { session, refresh } = request;
        let info = self
            .sessions
            .get(session)
            .await
            .map_err(|error| wire(&error))?;
        if !info.status.is_live() {
            return Ok(PullRequestView {
                access: ForgeAccess::Ended,
                pull_request: None,
                feedback: self.feedback_on(session).await,
            });
        }
        let held = if refresh {
            self.check_owner(connection.principal(), session).await?;
            let person = connection.principal().agent().is_none();
            self.forges
                .ask(|reply| Command::Refresh {
                    session,
                    person,
                    reply: Some(reply),
                })
                .await?
        } else {
            self.forges
                .ask(|reply| Command::Get { session, reply })
                .await?
        };
        Ok(held.unwrap_or(PullRequestView {
            access: ForgeAccess::NotPlaced,
            pull_request: None,
            feedback: self.feedback_on(session).await,
        }))
    }

    /// `pr.open`: a person's call pushes the branch first; an agent's
    /// never does.
    pub(crate) async fn open_pull_request(
        &self,
        connection: &Connection,
        request: OpenPullRequest,
    ) -> Result<OpenOutcome, WireError> {
        let push = connection.principal().agent().is_none();
        self.forges.open(&self.sessions, push, request).await
    }

    /// `pr.merge` (a person's; agents are refused by the policy).
    pub(crate) async fn merge_pull_request(
        &self,
        _: &Connection,
        request: MergePullRequest,
    ) -> Result<MergeOutcome, WireError> {
        self.forges.merge(&self.sessions, request).await
    }

    /// `pr.feedback`: kept in the store, then the watcher told. A terminal
    /// session has no queue to send it to.
    pub(crate) async fn set_feedback(
        &self,
        _: &Connection,
        request: SetFeedback,
    ) -> Result<SessionPullRequest, WireError> {
        let SetFeedback { session, enabled } = request;
        let info = self
            .sessions
            .get(session)
            .await
            .map_err(|error| wire(&error))?;
        if matches!(info.spec.mode, SessionMode::Terminal { .. }) {
            return Err(WireError::new(
                ErrorCode::Unsupported,
                "a terminal session has no queue to send feedback to",
            ));
        }
        self.forges
            .set_feedback(session, enabled)
            .await
            .map_err(|error| {
                tracing::warn!(%session, %error, "feedback could not be kept");
                WireError::new(ErrorCode::Internal, "the feedback switch could not be kept")
            })?;
        let held = self
            .forges
            .ask(|reply| Command::Feedback {
                session,
                on: enabled,
                reply,
            })
            .await?;
        let access = if info.status.is_live() {
            ForgeAccess::NotPlaced
        } else {
            ForgeAccess::Ended
        };
        Ok(held.unwrap_or(SessionPullRequest {
            access,
            summary: None,
            feedback: enabled,
        }))
    }

    /// Whether `session` has feedback on, as stored (off when it cannot be
    /// read).
    async fn feedback_on(&self, session: SessionId) -> bool {
        self.forges.feedback_on(session).await
    }

    /// Tell the watcher a session started (or started again) for
    /// `principal`, when it is placed in a project or a worktree.
    pub(super) fn watch_session(&self, handle: &SessionHandle, principal: &Principal) {
        let info = handle.info();
        let Some(place) = info.spec.place.as_deref() else {
            return;
        };
        let (Some(project), Some(folder)) = (self.forges.project_of(place), info.spec.cwd.clone())
        else {
            return;
        };
        self.forges.tell(Command::Started(Box::new(Started {
            handle: handle.clone(),
            folder,
            project,
            person: principal.agent().is_none(),
        })));
    }

    /// Tell the watcher a session was archived or deleted.
    pub(super) fn unwatch_session(&self, session: SessionId) {
        self.forges.tell(Command::Forget(session));
    }
}
