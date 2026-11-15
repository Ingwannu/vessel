//! ADR 0077 bounded capture orchestration; restore calls remain pending.
//!
//! Candidate workers cannot publish. The coordinator owns deadline fencing and
//! keeps root/worker/quota admission until cleanup, even after the actor resumes.

#[cfg(target_os = "linux")]
pub(crate) mod backend;
#[cfg(target_os = "linux")]
pub(crate) mod bridge;
mod capture;
mod quota;
mod runner;
#[cfg(test)]
mod runner_tests;
mod workers;

#[cfg(test)]
mod tests;
