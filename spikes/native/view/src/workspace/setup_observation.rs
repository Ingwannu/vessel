//! Read-only setup backing observations, with cancellation as a distinct receipt.
//! Canonical scalar values arrived through the announced package base. No RPC
//! or workspace start authority is supplied by this display model.
use crate::connection::ConnectionContext;
use crate::workspace::git::{Reading, RequestKey, RequestKeys};
use crate::workspace::problem::Problem;
use std::collections::BTreeMap;
use vessel_proto::workspace::{
    CancelSetup, CancelSetupOutcome, SetupCommandIndex, SetupCommandState, SetupProgress,
    SetupProgressReply, SetupRunState,
};
use vessel_proto::{SetupRunId, WorktreeId};

/// A backing observation address grants no canonical workspace start permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    /// Original actor admission; never resolved from the later selection.
    pub context: ConnectionContext,
    /// Backing history identity, not workspace start authority.
    pub worktree: WorktreeId,
}
/// A concrete reviewed backend supplies this negotiated surface, defaulting off.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Support {
    /// No reviewed producer or assigned observation feature.
    #[default]
    Unavailable,
    /// Structured read is available; no cancellation permission.
    Observe,
    /// The reviewed backend admits cancellation independently.
    ObserveAndCancel,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Expansion {
    Open,
    Closed,
}
/// Correlated display state; resource ownership stays on the node.
#[derive(Debug, Default)]
pub struct Model {
    keys: RequestKeys,
    /// Exact displayed backing and admission.
    pub address: Option<Address>,
    /// Capability supplied by the backend, never guessed from protocol numbers.
    pub support: Support,
    /// A pending or retained read cannot authorize a cancellation.
    pub read: Reading<SetupProgressReply>,
    retained: Option<SetupProgressReply>,
    /// Safe failure projection without peer diagnostic strings.
    pub failure: Option<Problem>,
    /// Request receipt, separate from command or process completion.
    pub cancellation: Option<CancelSetupOutcome>,
    pending_cancel: Option<(RequestKey, SetupRunId)>,
    requested: Option<SetupRunId>,
    run: Option<SetupRunId>,
    expansion: BTreeMap<u32, Expansion>,
}
impl Model {
    /// Replace displayed ownership without resetting the lifetime ticket issuer.
    pub fn select(&mut self, address: Option<Address>, support: Support) {
        self.address = address;
        self.support = support;
        self.read = Reading::Empty;
        self.retained = None;
        self.failure = None;
        self.cancellation = None;
        self.pending_cancel = None;
        self.requested = None;
        self.run = None;
        self.expansion.clear();
    }
    /// Start one read only when an explicit producer supports it.
    pub fn begin_read(&mut self) -> Option<RequestKey> {
        if self.address.is_none()
            || self.support == Support::Unavailable
            || self.read.outstanding().is_some()
        {
            return None;
        }
        let key = self.keys.issue().ok()?;
        self.read.start(key);
        Some(key)
    }
    /// Settle only the original address and outstanding read ticket.
    pub fn accept_read(
        &mut self,
        address: &Address,
        key: RequestKey,
        result: Result<SetupProgressReply, Problem>,
    ) -> bool {
        if self.address.as_ref() != Some(address) {
            return false;
        }
        // Full DTO resource validation remains the reviewed producer/decoder's
        // responsibility. No network adapter consumes these staging values.
        let result = result.and_then(|reply| {
            if reply
                .progress
                .as_ref()
                .is_some_and(|snapshot| snapshot.worktree != address.worktree)
            {
                Err(Problem::Gone(String::new()))
            } else {
                Ok(reply)
            }
        });
        let accepted = self.read.accept(key, result);
        if accepted && self.read.ready().is_some() {
            self.retained = None;
        }
        if accepted && let Some(reply) = self.read.ready() {
            let run = reply.progress.as_ref().map(|snapshot| snapshot.cursor.run);
            if self.run != run {
                self.run = run;
                self.expansion.clear();
                self.pending_cancel = None;
                self.requested = None;
                self.cancellation = None;
            }
        }
        accepted
    }
    /// Display retained facts without granting a write from them.
    #[must_use]
    pub fn snapshot(&self) -> Option<&SetupProgress> {
        self.read
            .shown()
            .or(self.retained.as_ref())?
            .progress
            .as_ref()
    }
    /// Prefer the running row; otherwise reveal the last attempted row.
    /// Planned waiting rows must not hide the command that actually failed.
    #[must_use]
    pub fn automatic(&self) -> Option<SetupCommandIndex> {
        let rows = &self.snapshot()?.commands;
        rows.iter()
            .find(|row| row.state == SetupCommandState::Running)
            .or_else(|| {
                rows.iter()
                    .rev()
                    .find(|row| row.state != SetupCommandState::Waiting)
            })
            .map(|row| row.index)
    }
    /// Manual collapse persists for this invocation despite later reads.
    #[must_use]
    pub fn expanded(&self, index: SetupCommandIndex) -> bool {
        match self.expansion.get(&index.get()) {
            Some(Expansion::Open) => true,
            Some(Expansion::Closed) => false,
            None => self.automatic() == Some(index),
        }
    }
    /// A stale row cannot change a replacement invocation.
    pub fn toggle(&mut self, run: SetupRunId, index: SetupCommandIndex) {
        if self.run != Some(run)
            || !self
                .snapshot()
                .is_some_and(|snapshot| snapshot.commands.iter().any(|row| row.index == index))
        {
            return;
        }
        let choice = if self.expanded(index) {
            Expansion::Closed
        } else {
            Expansion::Open
        };
        let _previous = self.expansion.insert(index.get(), choice);
    }
    /// Only a fresh running read and admitted backend can offer cancellation.
    #[must_use]
    pub fn can_cancel(&self) -> bool {
        self.support == Support::ObserveAndCancel
            && self.pending_cancel.is_none()
            && self
                .read
                .ready()
                .and_then(|reply| reply.progress.as_ref())
                .is_some_and(|snapshot| {
                    snapshot.state == SetupRunState::Running
                        && self.requested != Some(snapshot.cursor.run)
                })
    }
    /// Capture the invocation seen by the button, without retargeting a new run.
    pub fn begin_cancel(&mut self, run: SetupRunId) -> Option<(RequestKey, CancelSetup)> {
        if self.run != Some(run) || !self.can_cancel() {
            return None;
        }
        let snapshot = self.read.ready()?.progress.as_ref()?;
        let request = CancelSetup {
            worktree: snapshot.worktree,
            run: snapshot.cursor.run,
        };
        let key = self.keys.issue().ok()?;
        self.pending_cancel = Some((key, request.run));
        self.cancellation = None;
        self.failure = None;
        Some((key, request))
    }
    /// Acknowledge a request; never synthesize a stopped process or command.
    pub fn accept_cancel(
        &mut self,
        address: &Address,
        key: RequestKey,
        run: SetupRunId,
        result: Result<CancelSetupOutcome, Problem>,
    ) -> bool {
        if self.address.as_ref() != Some(address)
            || self.pending_cancel != Some((key, run))
            || self.run != Some(run)
        {
            return false;
        }
        self.pending_cancel = None;
        self.retained = self.read.shown().cloned();
        self.read = Reading::Empty;
        match result {
            Ok(outcome) => {
                if outcome == CancelSetupOutcome::Requested {
                    self.requested = Some(run);
                }
                self.cancellation = Some(outcome);
            }
            Err(problem) => self.failure = Some(problem),
        }
        // Requested does not modify the observed run or any command result.
        true
    }
}
/// Match setup-panel.tsx's per-line final-CR-segment presentation. This is not
/// the legacy streaming SetupLog overwrite emulator and preserves no fake grid.
#[must_use]
pub fn output(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .split('\n')
        .map(|line| line.rsplit('\r').next().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Paseo setup-panel.tsx's whole-unit format; no wall clock or floating point.
#[must_use]
pub fn elapsed(value: vessel_proto::DurationMs) -> String {
    const SECOND: std::time::Duration = std::time::Duration::from_secs(1);
    const MINUTE: std::time::Duration = std::time::Duration::from_secs(60);
    let value = value.as_duration();
    if value < SECOND {
        return format!("{}ms", value.as_millis());
    }
    if value < MINUTE {
        return format!("{}s", value.as_secs());
    }
    let minutes = value.as_secs() / MINUTE.as_secs();
    let seconds = value.as_secs() % MINUTE.as_secs();
    format!("{minutes}m {seconds}s")
}

#[cfg(test)]
mod tests;
