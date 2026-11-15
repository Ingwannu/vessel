//! Provider catalogue/configuration owners (ADR 0081). Method-table rows remain
//! pending until integrator assignment; unused host seams remain test-staged.

mod budget;
mod cache;
mod configuration;
mod failure;
mod lease;
mod manager;
mod owner;
mod producer;

pub use cache::CacheError as ProviderCacheError;
pub use configuration::ConfigurationFailure as ProviderConfigurationError;
pub use failure::ProviderTaskFailure;
pub(crate) use manager::{PreparedProviders, Providers};
pub use owner::OwnerError as ProviderDiscoveryError;

#[cfg(test)]
mod tests;
