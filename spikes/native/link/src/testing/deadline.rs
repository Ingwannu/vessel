//! Bounded memory-only actor tests: one outer safety net, named wait phases.
//!
//! [Decision Log]
//! - Purpose: a missing frame/tag must fail at its wait, never hang a runner.
//! - Constraint: the shared testkit prepares executable fakes; these tests use
//!   only an in-memory framed peer. V1's complete batch permits native Tokio
//!   guards without adding that unrelated root testkit dependency.
//! - Choice: Tokio's existing timeout, keeping every phase's five-second bound
//!   and a separate outer limit for the complete multi-phase regression.
//! - Cost: the outer label is the last safety net; phase names locate the wait.
use std::{future::Future, time::Duration};

const TEST_LIMIT: Duration = Duration::from_secs(30);

pub(crate) async fn within<T>(test: impl Future<Output = T>) -> T {
    tokio::time::timeout(TEST_LIMIT, test)
        .await
        .expect("memory-only connection regression exceeded its outer safety limit")
}

pub(crate) async fn phase<T>(name: &'static str, wait: impl Future<Output = T>) -> T {
    tokio::time::timeout(super::DEADLINE, wait)
        .await
        .unwrap_or_else(|_elapsed| panic!("connection regression phase timed out: {name}"))
}
