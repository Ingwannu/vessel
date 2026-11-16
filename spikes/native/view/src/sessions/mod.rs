//! Durable session directory rules; no network, rendering or provider startup.
//! Transcript search and provider-history import remain separate capabilities.
mod history;
mod lifecycle;
mod navigation;
mod policy;
#[cfg(test)]
mod tests;
pub use history::{DateBucket, HistoryPage, HostScope, history};
pub use lifecycle::{
    Lifecycle, LifecycleAction, LifecycleBusy, LifecycleTarget, LifecycleTicket, PendingLifecycle,
    satisfies_lifecycle,
};
pub use navigation::{
    Navigation, NavigationPurpose, NavigationTarget, NavigationTicket, SessionAddress,
};
pub use policy::{ClosePolicy, close_policy};
