//! ADR 0077 staged file mechanism; no auth/store/session orchestration.
//!
//! Captures use strict Git enumeration and regular-file guards. The caller
//! owns workspace exclusion and runs synchronous I/O off its async worker.

#[cfg(target_os = "linux")]
mod account;
#[cfg(target_os = "linux")]
mod archive;
#[cfg(target_os = "linux")]
mod capture;
#[cfg(target_os = "linux")]
mod content;
mod enumeration;
mod fingerprint;
#[cfg(target_os = "linux")]
mod inspect;
#[cfg(target_os = "linux")]
mod policy;
#[cfg(target_os = "linux")]
mod restore;
#[cfg(target_os = "linux")]
mod rooted;
#[cfg(target_os = "linux")]
mod scope;

#[cfg(test)]
mod tests;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("checkpoint filesystem access failed")]
    Io(#[from] std::io::Error),
    #[error("checkpoint git enumeration failed")]
    Git(#[from] crate::GitError),
    #[error("checkpoint operation refused: {0}")]
    Refused(#[from] vessel_proto::checkpoint::CheckpointError),
}
