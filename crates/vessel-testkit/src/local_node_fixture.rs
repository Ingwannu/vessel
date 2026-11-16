//! The feature-only real-node fixture, located before the caller's case clock.

use std::path::PathBuf;
use std::sync::OnceLock;

use crate::{LocateError, build, profile_dir};

/// Locate the freshly built managed-node causal fixture, never a shipped daemon.
///
/// The first call builds exactly vessel-node/local-node-fixture and its fixture
/// bin, using the existing nested-build owner. Call before the test case clock.
/// No node library dependency or normal product activation is introduced.
///
/// # Errors
/// Source build failure or a missing exact fixture binary is reported, not skipped.
pub fn local_node_fixture() -> Result<PathBuf, LocateError> {
    static BUILT: OnceLock<Result<PathBuf, LocateError>> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let directory = profile_dir()?;
            build(
                &directory,
                &[
                    "-p",
                    "vessel-node",
                    "--features",
                    "local-node-fixture",
                    "--bin",
                    "vessel-local-node-fixture",
                ],
            )?;
            let path = directory.join(format!(
                "vessel-local-node-fixture{}",
                std::env::consts::EXE_SUFFIX
            ));
            if path.is_file() {
                Ok(path)
            } else {
                Err(LocateError::Missing { path })
            }
        })
        .clone()
}
