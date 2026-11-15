//! Actual runner completion is separate from logical terminal status or revocation.
//! Only a runner can publish facts; unknown descendants/I/O never become success.
use tokio::sync::watch;
use vessel_proto::TerminalId;

/// A postcondition observed by the actual owning runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupFact {
    /// The runner positively observed this postcondition.
    Confirmed,
    /// The existing platform/runner does not establish this postcondition.
    Unconfirmed,
}
/// Immutable terminal cleanup facts, never inferred from an ended status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleanupReport {
    terminal: TerminalId,
    facts: Facts,
}
impl CleanupReport {
    /// The exact terminal whose actor completed.
    #[must_use]
    pub const fn terminal(self) -> TerminalId {
        self.terminal
    }
    /// Whether exit (or no successful spawn) was observed.
    #[must_use]
    pub const fn process(self) -> CleanupFact {
        self.facts.process
    }
    /// Whether the runner's descendant scope is known to be quiescent.
    #[must_use]
    pub const fn descendants(self) -> CleanupFact {
        self.facts.descendants
    }
    /// Whether owned I/O has completely settled, rather than timed out.
    #[must_use]
    pub const fn io(self) -> CleanupFact {
        self.facts.io
    }
    /// Whether final auxiliary-file cleanup finished successfully.
    #[must_use]
    pub const fn auxiliary(self) -> CleanupFact {
        self.facts.auxiliary
    }
    /// A necessary member postcondition, never workspace/path mutation authority.
    /// Registry lifecycle, generation and physical-root admission remain separate.
    #[must_use]
    pub fn is_confirmed(self) -> bool {
        self.facts.process == CleanupFact::Confirmed
            && self.facts.descendants == CleanupFact::Confirmed
            && self.facts.io == CleanupFact::Confirmed
            && self.facts.auxiliary == CleanupFact::Confirmed
    }
}
/// A dropped/panicked runner is not a successful cleanup acknowledgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupObservation {
    /// The actual actor returned after its owned cleanup, with explicit facts.
    Finished(CleanupReport),
    /// No final runner report exists; retain the resource and uncertainty.
    Abandoned {
        /// Exact terminal, never a latest registry replacement.
        terminal: TerminalId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Facts {
    pub(super) process: CleanupFact,
    pub(super) descendants: CleanupFact,
    pub(super) io: CleanupFact,
    pub(super) auxiliary: CleanupFact,
}
impl Facts {
    pub(super) const fn unconfirmed() -> Self {
        Self {
            process: CleanupFact::Unconfirmed,
            descendants: CleanupFact::Unconfirmed,
            io: CleanupFact::Unconfirmed,
            auxiliary: CleanupFact::Unconfirmed,
        }
    }
    /// No child or I/O worker was ever started by this actor.
    pub(super) const fn not_started() -> Self {
        Self {
            process: CleanupFact::Confirmed,
            descendants: CleanupFact::Confirmed,
            io: CleanupFact::Confirmed,
            auxiliary: CleanupFact::Unconfirmed,
        }
    }
}
/// The actor owns the sole sender. Its loss closes observation without success.
pub(super) struct Completion {
    terminal: TerminalId,
    sender: watch::Sender<Option<CleanupReport>>,
}
#[derive(Debug)]
pub(super) struct Progress {
    terminal: TerminalId,
    receiver: watch::Receiver<Option<CleanupReport>>,
}
pub(super) fn begin(terminal: TerminalId) -> (Completion, Progress) {
    let (sender, receiver) = watch::channel(None);
    (
        Completion { terminal, sender },
        Progress { terminal, receiver },
    )
}
impl Completion {
    /// Call after the actor future returned, so its owned resources were dropped.
    pub(super) fn finish(self, facts: Facts) {
        let _old = self.sender.send_replace(Some(CleanupReport {
            terminal: self.terminal,
            facts,
        }));
    }
}
impl Progress {
    pub(super) async fn observe(&self) -> CleanupObservation {
        let mut receiver = self.receiver.clone();
        let report = match receiver.wait_for(Option::is_some).await {
            Ok(report) => *report,
            Err(_) => None,
        };
        match report {
            Some(report) => CleanupObservation::Finished(report),
            None => CleanupObservation::Abandoned {
                terminal: self.terminal,
            },
        }
    }
}

#[cfg(test)]
mod tests;
