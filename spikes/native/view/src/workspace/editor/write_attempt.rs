//! A local wait ending does not prove that the remote filesystem effect ended.
//! Keep one original obligation; reconnect/read/hash equality cannot clear it.

/// One original write's effect, independent from whether its draft was undone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteState {
    /// Waiting for its original result.
    Pending,
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
