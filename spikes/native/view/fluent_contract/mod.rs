//! Pure Fluent AST contracts, shared by host generation and owned resources.
//! No folders, locale registry, I/O, runtime bundles or toolkit types belong here.
//! Callers parse with fluent-syntax and retain namespace/coverage policy.
mod expand;
mod pattern;
mod types;
mod validation;

pub(crate) use expand::expanded;
pub(crate) use pattern::{PatternRole, scan};
pub(crate) use types::{Contract, ContractError, Key, Kind, Name, Reference, Shape};
pub(crate) use validation::{VariableCoverage, compare};

#[cfg(test)]
mod tests;
