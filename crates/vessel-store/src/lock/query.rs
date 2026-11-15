//! Keep the public store snapshot path while delegating only its OS query.
//! Data-directory layout, owner records and lock acquisition remain in store.

pub use vessel_proc::lock::KernelLock;
#[cfg(target_os = "linux")]
pub(super) use vessel_proc::lock::observe;
