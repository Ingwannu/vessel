//! Files-only restore pauses actor input/queue without changing conversation state.
//! The node still owns physical exclusion, persistence, deadlines and filesystem I/O.
mod pause;
pub use pause::CheckpointPause;
pub(crate) use pause::State;
#[cfg(test)]
mod seam;
