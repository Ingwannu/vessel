//! After a self-update on Windows, the "Installed apps" entry the installer
//! wrote (HKCU) is told the new version, so Settings does not keep showing
//! the build the installer put there.
//!
//! Only an installed copy is touched: the entry's `InstallLocation` must be
//! the directory the updated executable is in. A copy from the zip, kept
//! anywhere else, has no entry of its own and is left alone, as is any
//! other installation's entry. A failure here is logged and changes
//! nothing else: the update itself is already in place.

use std::path::Path;

use crate::label::Label;

/// The key `windows/installer.nsi` writes (its `UNINSTALL_KEY`).
pub(crate) const UNINSTALL_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Vessel";

/// Set `DisplayVersion` under `key` (in HKCU) to `label` when the entry's
/// `InstallLocation` is `exe`'s directory. `Ok(false)` when there is no
/// such entry or it belongs to another directory.
pub(crate) fn record_version(
    key: &str,
    exe: &Path,
    label: &Label,
) -> windows_registry::Result<bool> {
    let Ok(entry) = windows_registry::CURRENT_USER.open(key) else {
        return Ok(false);
    };
    let Ok(location) = entry.get_string("InstallLocation") else {
        return Ok(false);
    };
    let Some(dir) = exe.parent() else {
        return Ok(false);
    };
    if !same_directory(Path::new(&location), dir) {
        return Ok(false);
    }
    let writable = windows_registry::CURRENT_USER
        .options()
        .read()
        .write()
        .open(key)?;
    writable.set_string("DisplayVersion", label.to_string())?;
    Ok(true)
}

/// Whether two paths name one directory (Windows paths compare without
/// regard to case; both are resolved when they can be).
fn same_directory(left: &Path, right: &Path) -> bool {
    let resolve = |path: &Path| {
        std::fs::canonicalize(path)
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .trim_end_matches('\\')
            .to_lowercase()
    };
    resolve(left) == resolve(right)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A key of the test's own, never the installer's.
    fn test_key() -> String {
        format!(r"Software\VesselUpdaterTest-{}", std::process::id())
    }

    #[test]
    fn the_installed_copys_entry_learns_the_new_version_and_others_do_not() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("Vessel.exe");
        std::fs::write(&exe, b"exe").unwrap();
        let key = test_key();
        let entry = windows_registry::CURRENT_USER.create(&key).unwrap();
        entry
            .set_string("InstallLocation", dir.path().to_string_lossy())
            .unwrap();
        entry.set_string("DisplayVersion", "0.0.0.0.3").unwrap();

        let label: Label = "0.0.0.0.4".parse().unwrap();
        assert!(record_version(&key, &exe, &label).unwrap());
        assert_eq!(entry.get_string("DisplayVersion").unwrap(), "0.0.0.0.4");

        // A copy somewhere else (the zip's) leaves the entry alone.
        let elsewhere = tempfile::tempdir().unwrap();
        let other: Label = "0.0.0.0.5".parse().unwrap();
        assert!(!record_version(&key, &elsewhere.path().join("Vessel.exe"), &other).unwrap());
        assert_eq!(entry.get_string("DisplayVersion").unwrap(), "0.0.0.0.4");

        windows_registry::CURRENT_USER.remove_tree(&key).unwrap();
        // No entry at all: nothing to do, no error.
        assert!(!record_version(&key, &exe, &label).unwrap());
    }
}
