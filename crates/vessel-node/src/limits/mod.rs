//! Credential-use policy and latest snapshots, owned by one bounded actor.
//!
//! [Decision Log]
//! - Purpose: an RPC cancellation must not leave a saved opt-in unapplied,
//!   or release admission while its blocking credential read still runs.
//! - Choice: one owner serializes policy writes and starts at most two
//!   probe jobs. A cancelled job is drained, never aborted; its token
//!   prevents HTTP after a queued file read and prevents publication.
//! - Alternatives: caller-owned settings writes can outlive cancellation
//!   without applying their result; a caller-owned semaphore releases too
//!   soon. Reworking unrelated node settings is outside this feature.
//! - Cost: a bounded RAM cache is lost at restart; saved sign-ins and
//!   snapshots never enter the database or policy file.
mod notice;
pub(crate) mod policy;
use chrono::Utc;
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_harness::{
    LimitBinding, LimitContext, LimitLoginIdentity, LimitProbe, LimitProbeResult,
};
use vessel_proto::{
    ErrorCode, WireError,
    limits::{
        AccountLimitState, ExternalLookup, LimitAccount, LimitFailure, LimitRefresh, LimitSnapshot,
        UsageLimits,
    },
    usage::CostVisibility,
};
use vessel_store::DataDir;

const COMMANDS: usize = 32;
const SNAPSHOTS: usize = 256;
const PROBES: usize = 2;
#[derive(Debug, Clone)]
pub(crate) struct Limits {
    pub(crate) sources: crate::usage_reports::Sources,
    commands: mpsc::Sender<Command>,
}
#[derive(Debug)]
enum Command {
    Observe {
        reply: oneshot::Sender<policy::Observation>,
    },
    Read {
        account: LimitAccount,
        binding: Result<LimitBinding, LimitFailure>,
        refresh: LimitRefresh,
        reply: oneshot::Sender<Result<UsageLimits, WireError>>,
    },
    Policy {
        policy: ExternalLookup,
        reply: oneshot::Sender<Result<ExternalLookup, WireError>>,
    },
}
impl Limits {
    /// The node's limits service from its configuration: the node
    /// assembly's one step for limits (protocol 23).
    pub(crate) async fn configured(
        config: &crate::Config,
        stop: CancellationToken,
        tasks: &TaskTracker,
    ) -> Self {
        let context = LimitContext::new();
        let mut limits = Self::start(
            config.data_dir.clone(),
            config.limits.clone(),
            stop.clone(),
            tasks,
            context.clone(),
        )
        .await;
        limits.sources = crate::usage_reports::Sources::start(config, stop, tasks, context).await;
        limits
    }

    pub(crate) async fn start(
        dir: DataDir,
        probe: LimitProbe,
        stop: CancellationToken,
        tasks: &TaskTracker,
        context: LimitContext,
    ) -> Self {
        let file = dir.clone();
        let policy = if let Ok(Ok(policy)) =
            tokio::task::spawn_blocking(move || file.read_usage_lookup()).await
        {
            policy
        } else {
            tracing::warn!("saved usage lookup policy could not be read; external lookup is off");
            ExternalLookup::Off
        };
        let (commands, receiver) = mpsc::channel(COMMANDS);
        let owner = Owner {
            dir,
            policy,
            probe,
            cache: Vec::new(),
            generation: CancellationToken::new(),
            jobs: JoinSet::new(),
            context,
        };
        drop(tasks.spawn(owner.run(receiver, stop)));
        Self {
            commands,
            sources: crate::usage_reports::Sources::default(),
        }
    }
    pub(crate) async fn read(
        &self,
        account: LimitAccount,
        binding: Result<LimitBinding, LimitFailure>,
        refresh: LimitRefresh,
    ) -> Result<UsageLimits, WireError> {
        let (reply, response) = oneshot::channel();
        self.commands
            .try_send(Command::Read {
                account,
                binding,
                refresh,
                reply,
            })
            .map_err(|_full| busy())?;
        response.await.map_err(|_closed| stopped())?
    }
    pub(crate) async fn policy(&self, policy: ExternalLookup) -> Result<ExternalLookup, WireError> {
        let (reply, response) = oneshot::channel();
        self.commands
            .try_send(Command::Policy { policy, reply })
            .map_err(|_full| busy())?;
        response.await.map_err(|_closed| stopped())?
    }
}
#[derive(Debug)]
struct Entry {
    account: LimitAccount,
    binding: LimitBinding,
    identity: Option<LimitLoginIdentity>,
    state: AccountLimitState,
}
struct Owner {
    dir: DataDir,
    policy: ExternalLookup,
    probe: LimitProbe,
    cache: Vec<Entry>,
    generation: CancellationToken,
    jobs: JoinSet<Completed>,
    context: LimitContext,
}
struct Completed {
    account: LimitAccount,
    binding: LimitBinding,
    result: LimitProbeResult,
    token: CancellationToken,
    reply: oneshot::Sender<Result<UsageLimits, WireError>>,
}
impl Owner {
    async fn run(mut self, mut commands: mpsc::Receiver<Command>, stop: CancellationToken) {
        loop {
            tokio::select! {
                biased;
                () = stop.cancelled() => break,
                result = self.jobs.join_next(), if !self.jobs.is_empty() => if let Some(result) = result { match result { Ok(result) => self.completed(result), Err(_panic) => { self.generation.cancel(); self.generation = CancellationToken::new(); self.cache.clear(); tracing::error!("a usage limits worker ended unexpectedly"); } } },
                command = commands.recv() => match command {
                    Some(Command::Observe {reply}) => { drop(reply.send(policy::Observation {policy:self.policy,generation:self.generation.child_token()})); },
                    Some(Command::Read { account, binding, refresh, reply }) => self.read(account, binding, refresh, reply),
                    Some(Command::Policy { policy, reply }) => {
                        let result = self.change(policy).await;
                        // A caller may have cancelled; the saved decision has
                        // already been applied by its independent owner.
                        drop(reply.send(result));
                    }
                    None => break,
                }
            }
        }
        self.generation.cancel();
        // Admission remains held until every blocking read actually finishes.
        while let Some(result) = self.jobs.join_next().await {
            match result {
                Ok(result) => drop(result.reply.send(Err(stopped()))),
                Err(_panic) => {
                    tracing::error!("a usage limits worker ended unexpectedly during shutdown");
                }
            }
        }
        self.context.drain().await;
    }
    async fn change(&mut self, policy: ExternalLookup) -> Result<ExternalLookup, WireError> {
        if policy == ExternalLookup::Off {
            self.generation.cancel();
            self.generation = CancellationToken::new();
            self.cache.clear();
            self.policy = policy;
        }
        let dir = self.dir.clone();
        tokio::task::spawn_blocking(move || dir.write_usage_lookup(policy)).await.map_err(|_join| stopped())?
            .map_err(|_private| WireError::new(ErrorCode::Internal, "the usage lookup choice could not be saved; check the machine's data directory and try again"))?;
        if policy == ExternalLookup::On && self.policy != policy {
            self.generation.cancel();
            self.generation = CancellationToken::new();
        }
        self.policy = policy;
        Ok(policy)
    }
    fn read(
        &mut self,
        account: LimitAccount,
        binding: Result<LimitBinding, LimitFailure>,
        refresh: LimitRefresh,
        reply: oneshot::Sender<Result<UsageLimits, WireError>>,
    ) {
        let source = binding.as_ref().ok().map(LimitBinding::source);
        if self.policy == ExternalLookup::Off {
            drop(reply.send(Ok(self.report(account, source, AccountLimitState::Off))));
            return;
        }
        let binding = match binding {
            Ok(binding) => binding,
            Err(reason) => {
                drop(reply.send(Ok(self.report(
                    account,
                    source,
                    AccountLimitState::Failed {
                        reason,
                        retained: None,
                        retry_at: None,
                    },
                ))));
                return;
            }
        };
        let state = self
            .cache
            .iter()
            .find(|entry| entry.account == account && entry.binding == binding)
            .map_or(AccountLimitState::NotRequested, |entry| entry.state.clone());
        let cooling = matches!(&state, AccountLimitState::Failed { retry_at: Some(at), .. } if *at > Utc::now());
        if refresh == LimitRefresh::Cached
            || cooling
            || matches!(state, AccountLimitState::Refreshing { .. })
        {
            drop(reply.send(Ok(self.report(account, source, state))));
            return;
        }
        if self.jobs.len() >= PROBES {
            drop(reply.send(Err(busy())));
            return;
        }
        let retained = retained(&state);
        if let Some(entry) = self
            .cache
            .iter_mut()
            .find(|entry| entry.account == account && entry.binding == binding)
        {
            entry.state = AccountLimitState::Refreshing { retained };
        } else {
            if self.cache.len() >= SNAPSHOTS
                && let Some(index) = self
                    .cache
                    .iter()
                    .position(|entry| !matches!(entry.state, AccountLimitState::Refreshing { .. }))
            {
                let _old = self.cache.remove(index);
            }
            self.cache.push(Entry {
                account: account.clone(),
                binding: binding.clone(),
                identity: None,
                state: AccountLimitState::Refreshing { retained },
            });
        }
        let probe = self.probe.clone();
        let context = self.context.clone();
        let token = self.generation.child_token();
        drop(self.jobs.spawn(async move {
            let result = probe.read(&context, &binding, Utc::now(), &token).await;
            Completed {
                account,
                binding,
                result,
                token,
                reply,
            }
        }));
    }
    fn completed(&mut self, completed: Completed) {
        let Completed {
            account,
            binding,
            result,
            token,
            reply,
        } = completed;
        if !token.is_cancelled()
            && self.policy == ExternalLookup::On
            && let Some(entry) = self
                .cache
                .iter_mut()
                .find(|entry| entry.account == account && entry.binding == binding)
        {
            let old = if result.identity.is_some() && result.identity == entry.identity {
                retained(&entry.state)
            } else {
                None
            };
            entry.identity = result.identity;
            entry.state = match result.result {
                Ok(snapshot) => AccountLimitState::Ready { snapshot },
                Err(reason) => AccountLimitState::Failed {
                    reason,
                    retained: old,
                    retry_at: result.retry_at,
                },
            };
        }
        let state = if self.policy == ExternalLookup::Off {
            AccountLimitState::Off
        } else {
            self.cache
                .iter()
                .find(|entry| entry.account == account && entry.binding == binding)
                .map_or(AccountLimitState::NotRequested, |entry| entry.state.clone())
        };
        drop(reply.send(Ok(self.report(account, Some(binding.source()), state))));
    }
    fn report(
        &self,
        account: LimitAccount,
        source: Option<vessel_proto::limits::LimitLoginStore>,
        state: AccountLimitState,
    ) -> UsageLimits {
        let notice = match &state {
            AccountLimitState::Ready { snapshot } => notice::notice(snapshot, Utc::now()),
            AccountLimitState::Off
            | AccountLimitState::NotRequested
            | AccountLimitState::Refreshing { .. }
            | AccountLimitState::Failed { .. } => None,
        };
        UsageLimits {
            policy: self.policy,
            account,
            source,
            state,
            cost_visibility: CostVisibility::Permitted,
            notice,
        }
    }
}
fn retained(state: &AccountLimitState) -> Option<LimitSnapshot> {
    match state {
        AccountLimitState::Ready { snapshot } => Some(snapshot.clone()),
        AccountLimitState::Refreshing { retained } | AccountLimitState::Failed { retained, .. } => {
            retained.clone()
        }
        AccountLimitState::Off | AccountLimitState::NotRequested => None,
    }
}
fn busy() -> WireError {
    WireError::new(
        ErrorCode::Unavailable,
        "usage limits are busy; wait for the current check and try again",
    )
}
fn stopped() -> WireError {
    WireError::new(
        ErrorCode::Unavailable,
        "the machine is stopping; reconnect before checking usage limits",
    )
}

#[cfg(test)]
mod shared_admission_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) mod test_support;
