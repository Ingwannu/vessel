//! ADR 0077 staged orchestration. Quality B registration/wiring removes cfg(test).
//!
//! Candidate workers cannot publish. The coordinator owns deadline fencing and
//! keeps root/worker/quota admission until cleanup, even after the actor resumes.

#[cfg(target_os = "linux")]
pub(crate) mod backend;
mod capture;
mod quota;
mod runner;
#[cfg(test)]
mod runner_tests;

#[cfg(test)]
mod tests;
