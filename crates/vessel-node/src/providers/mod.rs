//! Provider catalogue/configuration owners (ADR 0081). Method-table rows remain
//! pending until integrator assignment; trusted host consumers share this cache.

mod budget;
mod cache;
mod configuration;
mod manager;
mod owner;
mod producer;

pub use cache::CacheError as ProviderCacheError;
pub use configuration::ConfigurationFailure as ProviderConfigurationError;
pub use manager::ProviderRegistrations;
pub(crate) use manager::{PreparedProviders, Providers};
pub use owner::Change as ProviderChange;
pub use owner::OwnerError as ProviderDiscoveryError;

#[cfg(test)]
mod tests;
