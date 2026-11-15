//! The existing platform-resolved registry read, not a second registry service.

/// Whether opening a catalog reads the node cache or explicitly fetches it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fetch {
    /// Read the node's current copy, which can still be unfetched.
    Cached,
    /// A person requested the node's ordinary public-index refresh.
    Refresh,
}

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::CatalogueError;
#[cfg(not(target_arch = "wasm32"))]
pub(super) use native::read;
