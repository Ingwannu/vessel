//! Generic selected-file kernel observations, without node data or policy.
//! A snapshot alone must never authorize a signal or process-tree cleanup.

#[cfg(target_os = "linux")]
mod kernel;

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

/// A kernel writer bound to a lock inode; it is never constructed from the record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelLock {
    process: NodeProcess,
    file: LockFileIdentity,
}
