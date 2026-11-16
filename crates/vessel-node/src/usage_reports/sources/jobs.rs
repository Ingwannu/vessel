//! Jobs keep their owner slot until the underlying local read actually finishes.
use super::{
    Completed, DateTime, Entry, ExternalLookup, Observation, Owner, PROBES, SOURCE_BUDGET, Source,
    SourceReportEntry, UsageFetchFailure, UsageSourceFailure, Utc, WAITERS, oneshot,
};
use vessel_harness::AccountUsageOutcome;
use vessel_proto::{
    limits::LimitFailure,
    usage_reports::{
        ReportLoginFailures, SourceReportState, SuccessfulUsageReport, UsageAttemptFailure,
        UsageProblem, UsageReportId,
    },
};
impl Owner {
    pub(super) fn refresh(
        &mut self,
        source: Source,
        global: Observation,
        reply: oneshot::Sender<Result<(), UsageFetchFailure>>,
    ) {
        if !global.still_current() || global.policy != ExternalLookup::On {
            let _closed = reply.send(Err(UsageFetchFailure::SourceRemoved));
            return;
        }
        self.prune(&global);
        if !self.enabled(source) {
            let _closed = reply.send(Ok(()));
            return;
        }
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.source == source)
            && entry.active
        {
            entry.waiters.retain(|waiter| !waiter.is_closed());
            if entry.waiters.len() >= WAITERS {
                let _closed = reply.send(Err(UsageFetchFailure::Busy));
            } else {
                entry.waiters.push(reply);
            }
            return;
        }
        if self.jobs.len() >= PROBES {
            let _closed = reply.send(Err(UsageFetchFailure::Busy));
            return;
        }
        let position =
            if let Some(position) = self.entries.iter().position(|entry| entry.source == source) {
                position
            } else {
                self.entries.push(Entry {
                    source,
                    epoch: uuid::Uuid::new_v4(),
                    stop: self.stop.child_token(),
                    global: global.clone(),
                    identity: None,
                    report: None,
                    failure: None,
                    active: false,
                    waiters: Vec::new(),
                });
                self.entries.len().saturating_sub(1)
            };
        let Some(entry) = self.entries.get_mut(position) else {
            let _closed = reply.send(Err(UsageFetchFailure::SourceRemoved));
            return;
        };
        if cooling(entry, Utc::now()) {
            let _closed = reply.send(Ok(()));
            return;
        }
        entry.epoch = uuid::Uuid::new_v4();
        entry.global = global;
        entry.active = true;
        entry.waiters.push(reply);
        entry.failure = None;
        if let Some(report) = &mut entry.report {
            let origin = match report.state {
                SourceReportState::Available { .. } => {
                    vessel_proto::usage_reports::UsageRefreshOrigin::Available
                }
                SourceReportState::NotRequested
                | SourceReportState::Refreshing { .. }
                | SourceReportState::Unavailable { .. }
                | SourceReportState::Failed { .. } => {
                    vessel_proto::usage_reports::UsageRefreshOrigin::Unavailable
                }
            };
            report.state = SourceReportState::Refreshing {
                origin,
                started_at: Some(Utc::now()),
                retained: success(&report.state),
            };
        }
        let source = entry.source;
        let epoch = entry.epoch;
        let stop = entry.stop.child_token();
        let probe = self.probe.clone();
        let context = self.context.clone();
        let global = entry.global.clone();
        drop(self.jobs.spawn(async move {
            let reading = probe.read(&context, source.builtin(), Utc::now(), &stop);
            tokio::pin!(reading);
            // Preserve node/source cancellation and physical permits while the
            // independent global owner revokes this job even if its RPC vanished.
            let (result, timed_out) = tokio::select! {biased;
                ()=global.cancelled()=>{stop.cancel();(reading.await,false)},
                result=&mut reading=>(result,false),
                ()=tokio::time::sleep(SOURCE_BUDGET)=>{stop.cancel();(reading.await,true)}
            };
            Completed {
                source,
                epoch,
                result,
                timed_out,
                at: Utc::now(),
            }
        }));
    }
    pub(super) fn completed(&mut self, done: Completed) {
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.source == done.source && entry.epoch == done.epoch)
        else {
            return;
        };
        entry.active = false;
        let current = !entry.stop.is_cancelled() && entry.global.still_current();
        let result = if current {
            settle(entry, done)
        } else {
            Err(UsageFetchFailure::SourceRemoved)
        };
        for waiter in entry.waiters.drain(..) {
            let _closed = waiter.send(result);
        }
    }
}
fn cooling(entry: &Entry, now: DateTime<Utc>) -> bool {
    entry
        .report
        .as_ref()
        .is_some_and(|report| match &report.state {
            SourceReportState::Unavailable { retry_at, .. }
            | SourceReportState::Failed { retry_at, .. } => retry_at.is_some_and(|at| at > now),
            SourceReportState::NotRequested
            | SourceReportState::Refreshing { .. }
            | SourceReportState::Available { .. } => false,
        })
}
fn success(state: &SourceReportState) -> Option<SuccessfulUsageReport> {
    match state {
        SourceReportState::Available { observed } => Some(observed.clone()),
        SourceReportState::Refreshing { retained, .. }
        | SourceReportState::Unavailable { retained, .. }
        | SourceReportState::Failed { retained, .. } => retained.clone(),
        SourceReportState::NotRequested => None,
    }
}
fn settle(entry: &mut Entry, done: Completed) -> Result<(), UsageFetchFailure> {
    let same = done.result.identity.is_some() && done.result.identity == entry.identity;
    let retained = if same {
        entry
            .report
            .as_ref()
            .and_then(|report| success(&report.state))
    } else {
        None
    };
    if done.result.identity.is_some() && !same {
        let id = UsageReportId::new(
            entry
                .source
                .name()
                .parse()
                .map_err(|_invalid| UsageFetchFailure::ChangedResponse)?,
            uuid::Uuid::new_v4()
                .to_string()
                .parse()
                .map_err(|_invalid| UsageFetchFailure::ChangedResponse)?,
        );
        let descriptor = entry
            .source
            .descriptor(
                ExternalLookup::On,
                vessel_proto::usage_reports::UsageSourceConsent::Off,
                true,
            )
            .map_err(|_invalid| UsageFetchFailure::ChangedResponse)?;
        entry.report = Some(SourceReportEntry::new(
            id,
            descriptor.label,
            descriptor.builtin,
            None,
            None,
            SourceReportState::NotRequested,
        ));
        entry.identity.clone_from(&done.result.identity);
    }
    let state = state(
        done.result.result,
        done.result.retry_at,
        done.at,
        retained,
        done.timed_out,
    );
    if let Some(report) = &mut entry.report {
        report.state = state;
    } else {
        let failure = match state {
            SourceReportState::Unavailable { problem, .. } => {
                UsageAttemptFailure::Unavailable { problem }
            }
            SourceReportState::Failed { reason, .. } => UsageAttemptFailure::Error { reason },
            SourceReportState::NotRequested
            | SourceReportState::Refreshing { .. }
            | SourceReportState::Available { .. } => {
                return Err(UsageFetchFailure::ChangedResponse);
            }
        };
        entry.failure = Some(UsageSourceFailure {
            source: entry
                .source
                .name()
                .parse()
                .map_err(|_invalid| UsageFetchFailure::ChangedResponse)?,
            failure,
        });
    }
    Ok(())
}
fn state(
    result: Result<AccountUsageOutcome, LimitFailure>,
    retry_at: Option<DateTime<Utc>>,
    at: DateTime<Utc>,
    retained: Option<SuccessfulUsageReport>,
    timed_out: bool,
) -> SourceReportState {
    let reason = if timed_out {
        UsageFetchFailure::TimedOut
    } else {
        match result {
            Ok(AccountUsageOutcome::Available(report)) => {
                return SourceReportState::Available {
                    observed: SuccessfulUsageReport { at, report },
                };
            }
            Ok(AccountUsageOutcome::Unavailable(problem)) => {
                return unavailable(problem, at, retry_at, retained);
            }
            Err(LimitFailure::Unreachable) => UsageFetchFailure::Unreachable,
            Err(LimitFailure::ChangedResponse) => UsageFetchFailure::ChangedResponse,
            Err(LimitFailure::Busy) => UsageFetchFailure::Busy,
            Err(
                reason @ (LimitFailure::NoLogin
                | LimitFailure::Expired
                | LimitFailure::Rejected
                | LimitFailure::Unsupported
                | LimitFailure::UnsafeLoginStore
                | LimitFailure::DifferentRoute
                | LimitFailure::Unassociated
                | LimitFailure::RateLimited),
            ) => return unavailable(UsageProblem::Login { reason }, at, retry_at, retained),
        }
    };
    SourceReportState::Failed {
        attempted_at: Some(at),
        reason,
        retry_at,
        retained,
        logins: ReportLoginFailures::default(),
    }
}
fn unavailable(
    problem: UsageProblem,
    at: DateTime<Utc>,
    retry_at: Option<DateTime<Utc>>,
    retained: Option<SuccessfulUsageReport>,
) -> SourceReportState {
    SourceReportState::Unavailable {
        attempted_at: Some(at),
        problem,
        retry_at,
        retained,
        logins: ReportLoginFailures::default(),
    }
}
