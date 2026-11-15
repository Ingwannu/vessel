//! A local wait ending does not prove that the remote filesystem effect ended.
//! Without original terminal evidence, reconnect/read/hash equality cannot clear
//! the one original obligation. Ended attempts permit a subsequent reconciliation.

/// One original write's effect, independent from whether its draft was undone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteState {
    /// Waiting for its original result.
    Pending,
    /// Original direct handler ended; a fresh later read must confirm its effect.
    Ended,
    /// Possibly dispatched; no terminal completion evidence remains available.
    Uncertain,
}
#[derive(Debug, Clone)]
pub(super) struct WriteAttempt {
    pub(super) sequence: u64,
    pub(super) text: String,
    pub(super) base: Option<vessel_proto::workspace::FileVersion>,
    pub(super) state: WriteState,
}

/// Recovery must not turn leaving the file into a retry of the failed write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FlushAfterRecovery {
    Automatic,
    NewIntentRequired,
}
