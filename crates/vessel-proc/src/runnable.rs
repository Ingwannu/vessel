//! Whether this account may run a file (ADR 0117): what a node checks
//! before it starts a shell or a terminal profile's program, so the person
//! reads "not found" rather than a spawn that fails.
//!
//! On Unix the kernel's own answer, `access(X_OK)` (Paseo's check): an
//! execute bit the account's owner, group and other class does not give
//! is a refusal, which a mode test alone would miss. On Windows, a file.

use std::path::Path;

/// Whether `path` is a file this process's account may run.
#[must_use]
pub fn may_run(path: &Path) -> bool {
    let file = std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file());
    #[cfg(unix)]
    {
        file && nix::unistd::access(path, nix::unistd::AccessFlags::X_OK).is_ok()
    }
    #[cfg(not(unix))]
    {
        file
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::PermissionsExt as _;

    use super::*;

    #[test]
    fn only_a_file_the_account_may_execute_runs() {
        // A folder of this test's own (the crate has no temporary-folder
        // dependency, and adding one would change the lock file).
        let folder = std::env::temp_dir().join(format!("vessel-proc-may-run-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let file = folder.join("tool");
        std::fs::write(&file, b"#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(!may_run(&file));
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(may_run(&file));
        assert!(!may_run(&folder));
        assert!(!may_run(&folder.join("missing")));
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
