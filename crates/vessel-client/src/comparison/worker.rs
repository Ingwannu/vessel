//! A single task serializes admission and owns input across dropped reply futures.
//! Sequential preparation/copy uses one active selection (below the bound of two);
//! acknowledged model turns themselves run independently on the node.
use super::endpoint::Endpoint;
use super::job::{Action, Command};
use super::{
    ComparisonInput, ComparisonPlan, Failure, JobError, JobStatus, Phase, ResourceTrail,
    SelectionIndex, Snapshot, TargetReport, TargetStatus,
};
use std::sync::Arc;
use tokio::sync::{mpsc, watch};
use tokio::time::Instant;
use vessel_proto::workspace::{CommitId, SetupState, WorktreeBase};
use vessel_proto::{EventSeq, SessionStatus};

pub(super) type Reports = watch::Sender<Arc<Snapshot>>;
pub(super) enum Pending {
    None,
    Approval,
    Setup {
        state: SetupState,
        cursor: u64,
        eligible: Instant,
    },
    Ready {
        status: SessionStatus,
        cursor: EventSeq,
        eligible: Instant,
    },
}
pub(super) struct Entry {
    pub report: TargetReport,
    pub pending: Pending,
}
impl Entry {
    pub(super) fn publish(&self, reports: &Reports) {
        reports.send_modify(|snapshot| {
            if let Some(target) = Arc::make_mut(snapshot)
                .targets
                .get_mut(self.report.index.get())
            {
                target.clone_from(&self.report);
            }
        });
    }
    pub(super) fn admitted(&mut self, phase: Phase, reports: &Reports) {
        self.report.phase = phase;
        self.report.status = TargetStatus::InFlight;
        self.publish(reports);
    }
    pub(super) fn failed(&mut self, failure: Failure, mutation: bool, reports: &Reports) {
        self.pending = Pending::None;
        self.report.phase = failure.phase;
        self.report.status = if mutation && failure.uncertain() {
            TargetStatus::Uncertain(failure)
        } else if failure.kind == super::FailureKind::Cancelled {
            TargetStatus::Cancelled
        } else {
            TargetStatus::Failed(failure)
        };
        self.publish(reports);
    }
}
pub(super) struct Worker {
    endpoint: Endpoint,
    plan: Plan,
    entries: Vec<Entry>,
    reports: Reports,
    prepared: Prepared,
    common: Option<ComparisonInput>,
}
struct Plan {
    project: vessel_proto::ProjectId,
    base: WorktreeBase,
    pinned: Option<CommitId>,
}
enum Prepared {
    New,
    Done,
}
impl Worker {
    pub(super) fn new(
        endpoint: Endpoint,
        plan: ComparisonPlan,
    ) -> (Self, watch::Receiver<Arc<Snapshot>>) {
        let entries: Vec<_> = plan
            .targets
            .into_iter()
            .enumerate()
            .map(|(index, (selection, branch))| Entry {
                report: TargetReport {
                    index: SelectionIndex(index),
                    selection: Arc::new(selection),
                    phase: Phase::Preflight,
                    status: TargetStatus::Planned,
                    resources: ResourceTrail {
                        branch,
                        worktree: None,
                        session: None,
                        copy: None,
                        attachments: Vec::new(),
                    },
                },
                pending: Pending::None,
            })
            .collect();
        let (reports, receiver) = watch::channel(Arc::new(Snapshot {
            destination: endpoint.destination,
            status: JobStatus::Active,
            targets: entries.iter().map(|entry| entry.report.clone()).collect(),
            input_rejected: None,
            input_bound: false,
        }));
        (
            Self {
                endpoint,
                entries,
                reports,
                prepared: Prepared::New,
                common: None,
                plan: Plan {
                    project: plan.project,
                    base: plan.base,
                    pinned: None,
                },
            },
            receiver,
        )
    }
    pub(super) async fn run(mut self, mut commands: mpsc::Receiver<Command>) {
        loop {
            if self.endpoint.cancelled() {
                break;
            }
            let command = tokio::select! {
                biased;
                _ = self.endpoint.cancel.changed() => break,
                command = commands.recv() => match command { Some(command) => command, None => break },
            };
            let result = self.command(command.action).await;
            // A dropped response receiver has no ownership over operation state.
            drop(
                command
                    .reply
                    .send(result.map(|()| self.reports.borrow().clone())),
            );
        }
        self.close();
    }
    async fn command(&mut self, action: Action) -> Result<(), JobError> {
        match action {
            Action::Prepare => self.prepare().await,
            Action::Start(input) => {
                if self.common.is_some() {
                    return Err(JobError::AlreadyBound);
                }
                if matches!(self.prepared, Prepared::New) {
                    return Err(JobError::Unprepared);
                }
                if let Err(failure) =
                    super::validate::shared(&mut self.endpoint, &self.entries, &input).await
                {
                    self.reports.send_modify(|snapshot| {
                        Arc::make_mut(snapshot).input_rejected = Some(failure);
                    });
                } else {
                    self.common = Some(input);
                    self.reports.send_modify(|snapshot| {
                        let snapshot = Arc::make_mut(snapshot);
                        snapshot.input_rejected = None;
                        snapshot.input_bound = true;
                    });
                    self.dispatch().await;
                }
            }
            Action::Continue(index) => {
                let entry = self
                    .entries
                    .get_mut(index.get())
                    .ok_or(JobError::Selection)?;
                if !matches!(
                    entry.report.status,
                    TargetStatus::WaitingSetup
                        | TargetStatus::WaitingReady
                        | TargetStatus::NeedsApproval(_)
                ) {
                    return Err(JobError::CannotContinue);
                }
                if let Some(base) = &self.plan.pinned {
                    super::prepare::target(
                        &mut self.endpoint,
                        self.plan.project,
                        base,
                        entry,
                        &self.reports,
                    )
                    .await;
                    if let Some(input) = &self.common {
                        super::copy::dispatch(&mut self.endpoint, entry, input, &self.reports)
                            .await;
                    }
                }
            }
        }
        Ok(())
    }
    async fn prepare(&mut self) {
        if matches!(self.prepared, Prepared::Done) {
            return;
        }
        // Consumed before IO: cancellation never reconstructs a create-capable plan.
        self.prepared = Prepared::Done;
        match super::prepare::preflight(
            &mut self.endpoint,
            self.plan.project,
            &self.plan.base,
            &self.entries,
        )
        .await
        {
            Err(failure) => {
                for entry in &mut self.entries {
                    entry.failed(failure.clone(), false, &self.reports);
                }
            }
            Ok(base) => {
                self.plan.pinned = Some(base.clone());
                for entry in &mut self.entries {
                    if self.endpoint.cancelled() {
                        break;
                    }
                    super::prepare::target(
                        &mut self.endpoint,
                        self.plan.project,
                        &base,
                        entry,
                        &self.reports,
                    )
                    .await;
                }
            }
        }
    }
    async fn dispatch(&mut self) {
        if let Some(input) = &self.common {
            for entry in &mut self.entries {
                if self.endpoint.cancelled() {
                    break;
                }
                super::copy::dispatch(&mut self.endpoint, entry, input, &self.reports).await;
            }
        }
    }
    fn close(&mut self) {
        for entry in &mut self.entries {
            match entry.report.status {
                TargetStatus::Started
                | TargetStatus::Failed(_)
                | TargetStatus::Uncertain(_)
                | TargetStatus::Cancelled => {}
                TargetStatus::Planned
                | TargetStatus::InFlight
                | TargetStatus::WaitingSetup
                | TargetStatus::WaitingReady
                | TargetStatus::NeedsApproval(_)
                | TargetStatus::Ready => {
                    entry.pending = Pending::None;
                    entry.report.status = TargetStatus::Cancelled;
                    entry.publish(&self.reports);
                }
            }
        }
        self.reports
            .send_modify(|snapshot| Arc::make_mut(snapshot).status = JobStatus::Cancelled);
    }
}
