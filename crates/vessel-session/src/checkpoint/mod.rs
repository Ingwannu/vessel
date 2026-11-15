//! Files-only restore pauses actor input/queue without changing conversation state.
//! The node still owns physical exclusion, persistence, deadlines and filesystem I/O.
mod actor;
mod hooks;
mod metadata;
mod pause;
mod restarting;
mod startup;
pub use hooks::{CheckpointAdvance, CheckpointHooks, CheckpointReceipt, CheckpointRequest};
pub use pause::CheckpointPause;
pub(crate) use pause::{Signal, State};
pub(crate) use restarting::RestartAdmission;

pub use startup::{
    START_ADMISSION_DEADLINE, StartupHooks, StartupPhase, StartupProgress, StartupRefusal,
    StartupRequest,
};
