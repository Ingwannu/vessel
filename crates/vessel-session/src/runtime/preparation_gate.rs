//! Caller-owned admission for command preparation. The host's existing arbiter
//! commits each effect; a token or cached policy alone is not fresh authority.

use std::fmt;

use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

/// Physical preparation effects admitted separately, before their dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparationEffect {
    /// A blocking executable/launch filesystem lookup.
    Lookup,
    /// A new native version process, never a session or turn.
    Version,
}

/// A refused effect has not been dispatched and must never fall back to ACP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PreparationRefusal {
    /// The original caller or node stopped before effect admission.
    #[error("agent preparation was cancelled before effect admission")]
    Cancelled,
    /// The original deadline passed; a newer deadline cannot revive this work.
    #[error("agent preparation expired before effect admission")]
    Expired,
    /// The original owner/generation no longer admits this preparation.
    #[error("agent preparation owner changed before effect admission")]
    Changed,
}

/// A host decision made under its existing synchronous admission arbiter.
#[derive(Debug)]
pub enum PreparationAdmission {
    /// Logical effect admission committed. Dispatch follows without an await;
    /// cancellation after this point cannot retroactively refuse that effect.
    Admitted,
    /// A temporary pause. Subscribe before the check to avoid a lost wakeup.
    Wait(watch::Receiver<()>),
    /// Terminal refusal for this unadmitted effect and its original owner.
    Refused(PreparationRefusal),
}

/// Live cancellation checks borrowed only for the host's synchronous commit.
/// The host calls `check` inside its arbiter, beside its deadline/owner checks.
#[derive(Clone, Copy)]
pub struct PreparationCancellation<'a> {
    pub(super) node: &'a CancellationToken,
    pub(super) caller: &'a CancellationToken,
}

impl PreparationCancellation<'_> {
    /// Check live sources; this is neither a permission grant nor a snapshot.
    ///
    /// # Errors
    /// Returns Cancelled when either original source has stopped.
    pub fn check(self) -> Result<(), PreparationRefusal> {
        if self.node.is_cancelled() || self.caller.is_cancelled() {
            Err(PreparationRefusal::Cancelled)
        } else {
            Ok(())
        }
    }
}

impl fmt::Debug for PreparationCancellation<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PreparationCancellation { .. }")
    }
}

/// Retains the caller's existing token and fresh host admission operation.
/// No task, quota, revision or policy registry is created here. A host must call
/// the supplied cancellation check under its existing arbiter; Admitted is the
/// logical commit, not a provisional cached check. The callback never awaits.
pub struct PreparationGate<F> {
    pub(super) cancelled: CancellationToken,
    pub(super) admit: F,
}

impl<F> PreparationGate<F> {
    /// Bind the original caller token and synchronous host admission callback.
    /// A temporary policy pause returns Wait, never permanent cancellation.
    pub fn new(cancelled: CancellationToken, admit: F) -> Self {
        Self { cancelled, admit }
    }
}

impl<F> fmt::Debug for PreparationGate<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PreparationGate { .. }")
    }
}

pub(super) type RuntimeAdmission =
    for<'a> fn(PreparationEffect, PreparationCancellation<'a>) -> PreparationAdmission;

pub(super) fn runtime_gate(token: CancellationToken) -> PreparationGate<RuntimeAdmission> {
    let admit: RuntimeAdmission = admit_live;
    PreparationGate::new(token, admit)
}

fn admit_live(
    _: PreparationEffect,
    cancellation: PreparationCancellation<'_>,
) -> PreparationAdmission {
    match cancellation.check() {
        Ok(()) => PreparationAdmission::Admitted,
        Err(reason) => PreparationAdmission::Refused(reason),
    }
}
