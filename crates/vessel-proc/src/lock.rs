//! Generic selected-file kernel observations, without node data or policy.
//! A snapshot alone must never authorize a signal or process-tree cleanup.

#[cfg(target_os = "linux")]
mod descriptor;
#[cfg(target_os = "linux")]
mod generation;
#[cfg(target_os = "linux")]
mod kernel;
#[cfg(target_os = "linux")]
mod record;

#[cfg(target_os = "linux")]
pub use kernel::observe;

/// A kernel-observed process id, separate from a potentially stale owner record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NodeProcess(u32);

impl NodeProcess {
    #[cfg(target_os = "linux")]
    const MINIMUM: u32 = 1;
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LockFileIdentity {
    major: u64,
    minor: u64,
    inode: u64,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct KernelWriter {
    process: NodeProcess,
    file: LockFileIdentity,
}

/// A kernel writer bound to a lock inode; it is never constructed from the record.
/// Linux also brackets descriptor possession with process-generation and
/// PID-namespace checks. This snapshot still never grants signal authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelLock {
    process: NodeProcess,
    file: LockFileIdentity,
    #[cfg(target_os = "linux")]
    descriptor: descriptor::Proof,
}
