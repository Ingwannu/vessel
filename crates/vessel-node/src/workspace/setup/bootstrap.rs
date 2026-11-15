//! Automatic terminal policy, staged behind the authoritative registry adapter.
//!
//! [Decision Log]
//! - Purpose: open approved terminal specifications after successful setup.
//! - Constraint: the legacy terminal place/cwd cannot identify a workspace owner.
//! - Alternatives: infer ownership from the path, or await the registry contract.
//! - Choice: a backend must claim the exact workspace and settle its durable start
//!   lease when the actor registers. It must revalidate that lease before input.
//! - Why: archive, trust changes and cancellation can race shell readiness.
//! - Effect: tests exercise policy without spawning shells; no production caller
//!   or new RPC is enabled before the registry adapter has passed review.

use std::time::Duration;

use tokio_util::sync::CancellationToken;
use vessel_proto::workspace::{FileTerminal, ProjectCommands, SetupRunState};
use vessel_proto::{SessionTitle, SetupRunId, TerminalBytes, TerminalId, WorkspaceId};

/// Paseo's first-output wait is bounded independently from setup command timeout.
const READY_LIMIT: Duration = Duration::from_millis(1500);

/// A backend refusal never implies the folder or existing resources were removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Failure {
    Blocked,
    Unknown,
    Unavailable,
    Spawn,
    Input,
}

/// A typed receipt for each attempted file-order terminal; no raw errors leak.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResultState {
    Sent(TerminalId),
    Failed(Failure),
    Cancelled,
}

/// The setup invocation must still be the current successful bootstrap of this
/// exact workspace. A prior success from the same folder cannot authorize input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Scope {
    workspace: WorkspaceId,
    run: SetupRunId,
}

/// The adapter owns shell scrubbing, caps, start claims and actor registration.
/// Dropping an unopened lease must release its proved no-spawn reservation.
/// `open` must settle a claimed start even if its caller disappears. A proved
/// no-spawn failure can release it; an uncertain spawn must retain its lease.
trait Backend {
    type Lease;

    async fn claim(&self, scope: Scope) -> Result<Self::Lease, Failure>;
    async fn open(
        &self,
        lease: &Self::Lease,
        name: Option<&SessionTitle>,
    ) -> Result<TerminalId, Failure>;
    /// Must cover already-received output as well as a newly subscribed event.
    async fn first_output(&self, terminal: TerminalId);
    /// Atomically revalidate lifecycle, ownership and generation before input.
    async fn input(
        &self,
        lease: &Self::Lease,
        terminal: TerminalId,
        bytes: TerminalBytes,
    ) -> Result<(), Failure>;
}

/// Approval precedes even the empty-setup/terminal-only branch. The caller holds
/// the exact parsed set which was approved; there is no checkout reread here.
fn approved_terminals(commands: &ProjectCommands) -> Option<&[FileTerminal]> {
    match commands {
        ProjectCommands::Listed {
            approved: true,
            terminals,
            ..
        } => Some(terminals),
        ProjectCommands::None => Some(&[]),
        ProjectCommands::Listed {
            approved: false, ..
        }
        | ProjectCommands::Invalid { .. } => None,
    }
}

/// Sequential file order is ADR0066's deliberate difference from Paseo's
/// Promise.all. Failed terminal starts do not discard the worktree or prevent
/// other independent terminal specifications from being attempted.
async fn run(
    backend: &impl Backend,
    scope: Scope,
    commands: &ProjectCommands,
    setup: SetupRunState,
    cancel: &CancellationToken,
) -> Option<Vec<ResultState>> {
    let terminals = approved_terminals(commands)?;
    if setup != SetupRunState::Succeeded {
        return None;
    }
    let mut results = Vec::with_capacity(terminals.len());
    for spec in terminals {
        let result = one(backend, scope, spec, cancel).await;
        results.push(result);
        if result == ResultState::Cancelled {
            break;
        }
    }
    Some(results)
}

async fn one(
    backend: &impl Backend,
    scope: Scope,
    spec: &FileTerminal,
    cancel: &CancellationToken,
) -> ResultState {
    if cancel.is_cancelled() {
        return ResultState::Cancelled;
    }
    let lease = match backend.claim(scope).await {
        Ok(lease) => lease,
        Err(failure) => return ResultState::Failed(failure),
    };
    if cancel.is_cancelled() {
        return ResultState::Cancelled;
    }
    let terminal = match backend.open(&lease, spec.name.as_ref()).await {
        Ok(terminal) => terminal,
        Err(failure) => return ResultState::Failed(failure),
    };
    tokio::select! {
        biased;
        () = cancel.cancelled() => return ResultState::Cancelled,
        _ready = tokio::time::timeout(READY_LIMIT, backend.first_output(terminal)) => {}
    }
    // Catch cancellation after an immediate readiness result, before shell input.
    if cancel.is_cancelled() {
        return ResultState::Cancelled;
    }
    let mut bytes = spec.command.as_str().as_bytes().to_vec();
    bytes.push(b'\r');
    match backend
        .input(&lease, terminal, TerminalBytes::new(bytes.into()))
        .await
    {
        Ok(()) => ResultState::Sent(terminal),
        Err(failure) => ResultState::Failed(failure),
    }
}

#[cfg(test)]
mod tests;
