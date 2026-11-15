//! Staged provider catalogue owner (ADR 0081). The wiring commit after the shared
//! method-table head removes cfg(test) in lib.rs. No product RPC reaches it yet.

mod cache;
mod configuration;
mod owner;

#[cfg(test)]
mod tests;
