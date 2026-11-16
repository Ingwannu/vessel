//! One owner serializes source consent, snapshots, generation fences and bounded fetch waiters.
mod jobs;
mod owner;
mod policy;
mod spec;
use crate::limits::policy::Observation;
use chrono::{DateTime, Utc};
use spec::Source;
use std::time::Duration;
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_harness::{
    AccountUsageIdentity, AccountUsageObservation, AccountUsageProbe, LimitContext,
};
use vessel_proto::{
    ErrorCode, WireError,
    limits::ExternalLookup,
    usage_reports::{
        ReportSourceDescriptors, ReportSourceGrants, SetUsageSourceLookup, SourceReportEntry,
        UsageFetchFailure, UsageSourceDescriptor, UsageSourceFailure,
    },
};
use vessel_store::DataDir;
const COMMANDS: usize = 32;
const PROBES: usize = 2;
// The saved owner uses the other shared slot during one aggregate refresh.
const BULK_REFRESH_CONCURRENCY: usize = 1;
const WAITERS: usize = 32;
const SOURCE_BUDGET: Duration = Duration::from_secs(20);
const RESPONSE_BUDGET: Duration = Duration::from_secs(21);
#[derive(Debug, Clone, Default)]
pub(crate) struct Sources(Option<mpsc::Sender<Command>>);
#[derive(Debug)]
pub(crate) struct Snapshot {
    pub(crate) sources: ReportSourceDescriptors,
    pub(crate) reports: Vec<SourceReportEntry>,
    pub(crate) failures: Vec<UsageSourceFailure>,
    generation: CancellationToken,
    global: Observation,
}
impl Snapshot {
    pub(crate) fn still_current(&self) -> bool {
        !self.generation.is_cancelled() && self.global.still_current()
    }
}
#[derive(Debug)]
enum Command {
    #[cfg(test)]
    CachedEntries {
        reply: oneshot::Sender<usize>,
    },
    #[cfg(test)]
    Drained {
        reply: oneshot::Sender<()>,
    },
    Snapshot {
        global: Observation,
        reply: oneshot::Sender<Result<Snapshot, WireError>>,
    },
    Refresh {
        source: Source,
        global: Observation,
        reply: oneshot::Sender<Result<(), UsageFetchFailure>>,
    },
    Set {
        request: SetUsageSourceLookup,
        global: Observation,
        reply: oneshot::Sender<Result<UsageSourceDescriptor, WireError>>,
    },
    Clear {
        reply: oneshot::Sender<()>,
    },
}
struct Entry {
    source: Source,
    epoch: uuid::Uuid,
    stop: CancellationToken,
    global: Observation,
    identity: Option<AccountUsageIdentity>,
    report: Option<SourceReportEntry>,
    failure: Option<UsageSourceFailure>,
    active: bool,
    waiters: Vec<oneshot::Sender<Result<(), UsageFetchFailure>>>,
}
struct Owner {
    dir: DataDir,
    probe: AccountUsageProbe,
    context: LimitContext,
    grants: ReportSourceGrants,
    /// What the consent file holds as far as this owner knows; `None` after
    /// an unreadable file or a failed save, so the next choice always writes.
    persisted: Option<ReportSourceGrants>,
    entries: Vec<Entry>,
    jobs: JoinSet<Completed>,
    stop: CancellationToken,
    legacy_supported: bool,
    policy_generation: CancellationToken,
    observed_global: Option<Observation>,
}
struct Completed {
    source: Source,
    epoch: uuid::Uuid,
    result: AccountUsageObservation,
    timed_out: bool,
    at: DateTime<Utc>,
}
impl Sources {
    #[cfg(test)]
    pub(crate) async fn drained(&self) {
        let (reply, response) = oneshot::channel();
        self.0
            .as_ref()
            .unwrap()
            .send(Command::Drained { reply })
            .await
            .unwrap();
        response.await.unwrap();
    }
    #[cfg(test)]
    pub(crate) async fn cached_entries(&self) -> usize {
        let (reply, response) = oneshot::channel();
        self.0
            .as_ref()
            .unwrap()
            .send(Command::CachedEntries { reply })
            .await
            .unwrap();
        response.await.unwrap()
    }
    pub(crate) async fn start(
        config: &crate::Config,
        stop: CancellationToken,
        tasks: &TaskTracker,
        context: LimitContext,
    ) -> Self {
        let dir = config.data_dir.clone();
        let persisted = tokio::task::spawn_blocking(move || dir.read_usage_sources())
            .await
            .ok()
            .and_then(Result::ok);
        // An unreadable or invalid file fails Off in memory (ADR 0042).
        let grants = persisted.clone().unwrap_or_default();
        let legacy_supported = config.limits.supported();
        let owner = Owner {
            dir: config.data_dir.clone(),
            probe: config.usage_sources.clone(),
            context,
            grants,
            persisted,
            entries: Vec::new(),
            jobs: JoinSet::new(),
            stop: stop.clone(),
            legacy_supported,
            policy_generation: stop.child_token(),
            observed_global: None,
        };
        let (commands, receiver) = mpsc::channel(COMMANDS);
        drop(tasks.spawn(owner.run(receiver)));
        Self(Some(commands))
    }
    pub(crate) async fn snapshot(&self, global: &Observation) -> Result<Snapshot, WireError> {
        if !global.still_current() {
            return Err(changed());
        }
        let Some(commands) = &self.0 else {
            return Ok(Snapshot {
                sources: spec::legacy(global.policy, false)?
                    .try_into()
                    .map_err(|_bound| changed())?,
                reports: Vec::new(),
                failures: Vec::new(),
                generation: CancellationToken::new(),
                global: global.clone(),
            });
        };
        let (reply, response) = oneshot::channel();
        commands
            .try_send(Command::Snapshot {
                global: global.clone(),
                reply,
            })
            .map_err(|_full| busy())?;
        let snapshot = response.await.map_err(|_closed| changed())??;
        if !snapshot.still_current() || !global.still_current() {
            return Err(changed());
        }
        Ok(snapshot)
    }
    pub(crate) async fn set(
        &self,
        request: SetUsageSourceLookup,
        global: &Observation,
    ) -> Result<UsageSourceDescriptor, WireError> {
        let commands = self.0.as_ref().ok_or_else(unsupported)?;
        let (reply, response) = oneshot::channel();
        commands
            .try_send(Command::Set {
                request,
                global: global.clone(),
                reply,
            })
            .map_err(|_full| busy())?;
        response.await.map_err(|_closed| changed())?
    }
    pub(crate) async fn clear(&self) -> Result<(), WireError> {
        let Some(commands) = &self.0 else {
            return Ok(());
        };
        let (reply, response) = oneshot::channel();
        commands
            .try_send(Command::Clear { reply })
            .map_err(|_full| busy())?;
        response.await.map_err(|_closed| changed())
    }
    pub(crate) async fn refresh_all(&self, global: &Observation) -> Vec<UsageSourceFailure> {
        use futures_util::{StreamExt as _, stream};
        stream::iter(Source::ALL)
            .map(|source| async move {
                self.refresh(source, global).await.err().and_then(|reason| {
                    Some(UsageSourceFailure {
                        source: source.name().parse().ok()?,
                        failure: vessel_proto::usage_reports::UsageAttemptFailure::Error { reason },
                    })
                })
            })
            .buffered(BULK_REFRESH_CONCURRENCY)
            .filter_map(async move |failure| failure)
            .collect()
            .await
    }
    async fn refresh(&self, source: Source, global: &Observation) -> Result<(), UsageFetchFailure> {
        let Some(commands) = &self.0 else {
            return Ok(());
        };
        let (reply, response) = oneshot::channel();
        commands
            .try_send(Command::Refresh {
                source,
                global: global.clone(),
                reply,
            })
            .map_err(|_full| UsageFetchFailure::Busy)?;
        tokio::time::timeout(RESPONSE_BUDGET, response)
            .await
            .map_err(|_elapsed| UsageFetchFailure::TimedOut)?
            .map_err(|_closed| UsageFetchFailure::SourceRemoved)?
    }
    pub(crate) async fn inspect(
        &self,
        id: &vessel_proto::usage_reports::UsageReportId,
        global: &Observation,
        refresh: vessel_proto::limits::LimitRefresh,
    ) -> Result<Option<SourceReportEntry>, WireError> {
        let before = self.snapshot(global).await?;
        let Some(report) = before.reports.into_iter().find(|report| report.id() == id) else {
            return Ok(None);
        };
        if refresh == vessel_proto::limits::LimitRefresh::Cached {
            return Ok(Some(report));
        }
        let Some(source) = Source::from_id(report.source()) else {
            return Err(changed());
        };
        self.refresh(source, global).await.map_err(|_safe| busy())?;
        Ok(self
            .snapshot(global)
            .await?
            .reports
            .into_iter()
            .find(|report| report.id() == id))
    }
}
fn changed() -> WireError {
    WireError::new(
        ErrorCode::Conflict,
        "usage sources changed; read their reports again",
    )
}
fn busy() -> WireError {
    WireError::new(
        ErrorCode::Unavailable,
        "source usage checks are busy; try again after current work finishes",
    )
}
fn unsupported() -> WireError {
    WireError::new(
        ErrorCode::Unsupported,
        "this source usage adapter is unavailable",
    )
}
#[cfg(test)]
mod tests;
