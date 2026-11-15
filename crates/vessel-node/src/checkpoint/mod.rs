//! ADR 0077 staged orchestration. Quality B registration/wiring removes cfg(test).
//!
//! Candidate workers cannot publish. The coordinator owns deadline fencing and
//! keeps root/worker/quota admission until cleanup, even after the actor resumes.

mod capture;
mod quota;

#[cfg(test)]
mod tests;
