//! Advisory waits stop on the first no-progress reply, retaining monotonic eligibility.
use super::endpoint::{Endpoint, READ};
use super::worker::{Entry, Pending, Reports};
use super::{Failure, FailureKind, Phase, TargetStatus};
use std::time::Duration;
use tokio::time::Instant;
use vessel_proto::workspace::{ReadSetupOutput, SetupState};
use vessel_proto::{AgentEvent, ReadEvents, SessionEvent};

const CADENCE: Duration = Duration::from_secs(1);
const SETUP_BUDGET: Duration = Duration::from_secs(600);

pub(super) async fn setup(endpoint: &mut Endpoint, entry: &mut Entry, reports: &Reports) {
    let Some(worktree) = entry.report.resources.worktree else {
        return;
    };
    let budget = super::endpoint::after(SETUP_BUDGET);
    loop {
        let Pending::Setup {
            state,
            cursor,
            eligible,
        } = &entry.pending
        else {
            return;
        };
        match state {
            SetupState::None | SetupState::Succeeded { .. } => {
                entry.pending = Pending::None;
                entry.report.status = TargetStatus::WaitingSetup;
                entry.publish(reports);
                return;
            }
            SetupState::Failed { .. } | SetupState::Unknown | SetupState::Skipped => {
                entry.failed(
                    Failure::new(Phase::Setup, FailureKind::Setup),
                    false,
                    reports,
                );
                return;
            }
            SetupState::Running { .. } => {}
        }
        if Instant::now() < *eligible || Instant::now() >= budget {
            return;
        }
        let cursor = *cursor;
        let state = state.clone();
        entry.admitted(Phase::Setup, reports);
        match endpoint
            .call(
                Phase::Setup,
                ReadSetupOutput {
                    worktree,
                    from: cursor,
                    wait: Some(30),
                },
                budget.min(super::endpoint::after(READ)),
            )
            .await
        {
            Ok(output) => {
                let progress = output.next > cursor || output.setup != state;
                entry.pending = Pending::Setup {
                    state: output.setup,
                    cursor: output.next,
                    eligible: super::endpoint::after(if progress {
                        Duration::ZERO
                    } else {
                        CADENCE
                    }),
                };
                entry.report.status = TargetStatus::WaitingSetup;
                entry.publish(reports);
                if !progress {
                    return;
                }
            }
            Err(failure) if failure.kind == FailureKind::Deadline => {
                entry.pending = Pending::Setup {
                    state,
                    cursor,
                    eligible: super::endpoint::after(CADENCE),
                };
                entry.report.status = TargetStatus::WaitingSetup;
                entry.publish(reports);
                return;
            }
            Err(failure) => {
                entry.failed(failure, false, reports);
                return;
            }
        }
    }
}
pub(super) async fn ready(endpoint: &mut Endpoint, entry: &mut Entry, reports: &Reports) {
    let Some(session) = entry.report.resources.session else {
        return;
    };
    let budget = super::endpoint::after(READ);
    loop {
        let Pending::Ready {
            status,
            cursor,
            eligible,
        } = &entry.pending
        else {
            return;
        };
        if !status.is_live() {
            entry.failed(
                Failure::new(Phase::Ready, FailureKind::AgentEnded),
                false,
                reports,
            );
            return;
        }
        if Instant::now() < *eligible || Instant::now() >= budget {
            return;
        }
        let cursor = *cursor;
        let old_status = status.clone();
        entry.admitted(Phase::Ready, reports);
        match endpoint
            .call(
                Phase::Ready,
                ReadEvents {
                    session,
                    from: cursor,
                    limit: 128,
                    wait: Some(30),
                },
                budget.min(super::endpoint::after(READ)),
            )
            .await
        {
            Ok(page) => {
                let (status, accepted) = readiness(&page.records, &old_status);
                if !status.is_live() {
                    entry.failed(
                        Failure::new(Phase::Ready, FailureKind::AgentEnded),
                        false,
                        reports,
                    );
                    return;
                }
                if accepted {
                    entry.pending = Pending::None;
                    entry.report.status = TargetStatus::Ready;
                    entry.publish(reports);
                    return;
                }
                let progress = page.next > cursor || status != old_status;
                entry.pending = Pending::Ready {
                    status,
                    cursor: page.next,
                    eligible: super::endpoint::after(if progress {
                        Duration::ZERO
                    } else {
                        CADENCE
                    }),
                };
                entry.report.status = TargetStatus::WaitingReady;
                entry.publish(reports);
                if !progress {
                    return;
                }
            }
            Err(failure) if failure.kind == FailureKind::Deadline => {
                entry.pending = Pending::Ready {
                    status: old_status,
                    cursor,
                    eligible: super::endpoint::after(CADENCE),
                };
                entry.report.status = TargetStatus::WaitingReady;
                entry.publish(reports);
                return;
            }
            Err(failure) => {
                entry.failed(failure, false, reports);
                return;
            }
        }
    }
}

fn readiness(
    records: &[vessel_proto::EventRecord],
    previous: &vessel_proto::SessionStatus,
) -> (vessel_proto::SessionStatus, bool) {
    let mut status = previous.clone();
    let mut accepted = false;
    for record in records {
        if let SessionEvent::Status { status: changed } = &record.event {
            status.clone_from(changed);
        }
        if let SessionEvent::Agent {
            event: AgentEvent::Ready { .. },
        } = &record.event
        {
            accepted = true;
        }
    }
    (status, accepted)
}
