//! Read-only preflight pins one commit before independent guarded creation.
use super::endpoint::{Endpoint, MUTATE, READ};
use super::worker::{Entry, Pending, Reports};
use super::{Failure, FailureKind, Phase, TargetStatus};
use std::sync::Arc;
use tokio::time::Instant;
use vessel_proto::workspace::{
    CommitId, CreateWorktree, CreateWorktreeOutcome, GetProject, ProjectState, ReadLog,
    SessionPlace, SetupChoice, WorkspaceRoot, WorktreeBase, WorktreeBranch,
};
use vessel_proto::{
    CreateSession, GetHarnessOptions, GetNode, ProjectId, ProtocolVersion, SessionMode, SessionSpec,
};

pub(super) async fn preflight(
    endpoint: &mut Endpoint,
    project: ProjectId,
    base: &WorktreeBase,
    entries: &[Entry],
) -> Result<CommitId, Failure> {
    let need = entries
        .iter()
        .fold(ProtocolVersion::new(10), |need, entry| {
            need.max(entry.report.selection.harness.since())
        });
    if endpoint.client.protocol() < need {
        return Err(Failure::new(Phase::Preflight, FailureKind::Protocol));
    }
    let node = endpoint
        .call(Phase::Preflight, GetNode, super::endpoint::after(READ))
        .await?;
    if node.protocol.max < need {
        return Err(Failure::new(Phase::Preflight, FailureKind::Protocol));
    }
    let info = endpoint
        .call(
            Phase::Preflight,
            GetProject {
                project,
                changed_from: None,
            },
            super::endpoint::after(READ),
        )
        .await?;
    if !info.git || info.state != ProjectState::Ready {
        return Err(Failure::new(Phase::Preflight, FailureKind::Project));
    }
    for entry in entries {
        let selected = &entry.report.selection;
        let options = endpoint
            .call(
                Phase::Preflight,
                GetHarnessOptions {
                    harness: selected.harness.clone(),
                    profile: selected.profile.clone(),
                },
                super::endpoint::after(READ),
            )
            .await?;
        // An unavailable adapter deliberately answers options successfully with
        // no capabilities. A successful RPC alone is not structured availability.
        if options.abilities.is_none() {
            return Err(Failure::new(Phase::Preflight, FailureKind::Provider));
        }
    }
    let page = endpoint
        .call(
            Phase::Preflight,
            ReadLog {
                root: WorkspaceRoot::Project { project },
                from: Some(base.clone()),
                skip: 0,
                limit: Some(1),
            },
            super::endpoint::after(READ),
        )
        .await?;
    page.commits
        .first()
        .map(|commit| commit.id.clone())
        .ok_or_else(|| Failure::new(Phase::Preflight, FailureKind::Project))
}

pub(super) async fn target(
    endpoint: &mut Endpoint,
    project: ProjectId,
    base: &CommitId,
    entry: &mut Entry,
    reports: &Reports,
) {
    match entry.report.status {
        TargetStatus::Planned | TargetStatus::NeedsApproval(_) => {
            create_worktree(endpoint, project, base, entry, reports).await;
        }
        TargetStatus::WaitingSetup | TargetStatus::WaitingReady => {}
        TargetStatus::InFlight
        | TargetStatus::Ready
        | TargetStatus::Started
        | TargetStatus::Failed(_)
        | TargetStatus::Uncertain(_)
        | TargetStatus::Cancelled => return,
    }
    if matches!(entry.pending, Pending::Setup { .. }) {
        super::observe::setup(endpoint, entry, reports).await;
    }
    if matches!(entry.report.status, TargetStatus::WaitingSetup)
        && matches!(entry.pending, Pending::None)
        && entry.report.resources.session.is_none()
    {
        create_session(endpoint, entry, reports).await;
    }
    if matches!(entry.pending, Pending::Ready { .. }) {
        super::observe::ready(endpoint, entry, reports).await;
    }
}
async fn create_worktree(
    endpoint: &mut Endpoint,
    project: ProjectId,
    base: &CommitId,
    entry: &mut Entry,
    reports: &Reports,
) {
    let request = CreateWorktree {
        project,
        branch: WorktreeBranch::New {
            name: Some(entry.report.resources.branch.clone()),
            base: Some(WorktreeBase::Commit { id: base.clone() }),
        },
        setup: SetupChoice::Run,
    };
    entry.pending = Pending::None;
    entry.admitted(Phase::WorktreeCreate, reports);
    match endpoint
        .call(
            Phase::WorktreeCreate,
            request,
            super::endpoint::after(MUTATE),
        )
        .await
    {
        Err(failure) => entry.failed(failure, true, reports),
        Ok(CreateWorktreeOutcome::NeedsApproval { commands }) => {
            entry.pending = Pending::Approval;
            entry.report.status = TargetStatus::NeedsApproval(Arc::new(commands));
            entry.publish(reports);
        }
        Ok(CreateWorktreeOutcome::Refused { reason }) => {
            let mut failure = Failure::new(Phase::WorktreeCreate, FailureKind::Worktree);
            failure.refusal = Some(reason);
            entry.failed(failure, false, reports);
        }
        Ok(CreateWorktreeOutcome::Created { worktree }) => {
            entry.report.resources.worktree = Some(worktree.id);
            entry.pending = Pending::Setup {
                state: worktree.setup,
                cursor: 0,
                eligible: Instant::now(),
            };
            entry.report.status = TargetStatus::WaitingSetup;
            entry.publish(reports);
        }
    }
}
async fn create_session(endpoint: &mut Endpoint, entry: &mut Entry, reports: &Reports) {
    let Some(worktree) = entry.report.resources.worktree else {
        return;
    };
    let selected = &entry.report.selection;
    let mut spec = SessionSpec::new(
        selected.harness.clone(),
        SessionMode::Structured {
            model: selected.model.clone(),
            resume: None,
            options: selected.options.clone(),
        },
    )
    .with_place(SessionPlace::Worktree { worktree });
    if let Some(profile) = &selected.profile {
        spec = spec.with_profile(profile.clone());
    }
    // The consumed setup continuation cannot reappear after a lost create reply.
    entry.pending = Pending::None;
    entry.admitted(Phase::SessionCreate, reports);
    match endpoint
        .call(
            Phase::SessionCreate,
            CreateSession { spec },
            super::endpoint::after(MUTATE),
        )
        .await
    {
        Err(failure) => entry.failed(failure, true, reports),
        Ok(session) => {
            entry.report.resources.session = Some(session.id);
            entry.pending = Pending::Ready {
                status: session.status,
                cursor: vessel_proto::EventSeq::ZERO,
                eligible: Instant::now(),
            };
            entry.report.status = TargetStatus::WaitingReady;
            entry.publish(reports);
        }
    }
}
