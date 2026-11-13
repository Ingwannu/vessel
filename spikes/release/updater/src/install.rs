//! Where this copy of the app is, and replacing it with a staged build.
//!
//! [Decision Log]
//! - Purpose: put a verified build in the running app's place, so the
//!   next start (a restart the owner clicks, or any later launch) runs it.
//! - Constraints: Windows will not overwrite or delete a running `.exe`,
//!   but will rename it; Unix replaces a file by `rename` atomically while
//!   the old inode stays mapped by the running process; the app must
//!   update without administrator rights (it is installed per user).
//! - Alternatives: Tauri and Zed on Windows run their installer again,
//!   silently, after the app exits (Zed adds a helper that waits for the
//!   process and moves files with rollback). That needs a second program
//!   and the installer as the update's artifact, and leaves the portable
//!   `.zip` copy unable to update. A copy-over on next start needs a
//!   launcher.
//! - Choice: the app is one executable on Windows and Linux. To replace
//!   it: on Windows, rename the running file aside (`<name>.old-<label>`),
//!   then rename the staged file into its place, and rename back if that
//!   fails; on Unix, make the staged file executable and rename it over
//!   the old one. Leftovers are removed at the next start.
//! - Why: no helper process, no installer run, no rights beyond the
//!   app's own directory; the same code updates the installed copy and the
//!   portable one; the swap cannot leave the app missing (a rename either
//!   happened or not, and the Windows path undoes its first rename).
//! - Cost: the Start menu's "Programs and Features" entry keeps the
//!   version the installer wrote until the next install (the app does not
//!   write the registry); an app in a location the user cannot write (a
//!   system-wide install) cannot update itself and says so; a bad build is
//!   not rolled back automatically (the `.old` file is removed at the next
//!   start; Zed does not roll back either).
//!
//! macOS (a bundle, a directory) is named here so the feed and the state
//! can describe it, but replacing a bundle is not built yet: it cannot be
//! run on the machines this is built on (`docs/plans/backend-m12-release.md`).

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::{label::Label, platform::Packaging, stage::STAGING_PREFIX, stage::Staged};

/// The most directory entries looked at when cleaning up, so an app kept
/// in a crowded folder (Downloads) does not make every start slow.
const MAX_CLEAN_SCAN: usize = 4096;

/// This copy of the app on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installation {
    packaging: Packaging,
    /// The file (an executable, an `AppImage`) or directory (a bundle) an
    /// update replaces.
    path: PathBuf,
    /// The executable to start after an update: `path` itself, or the
    /// binary inside the bundle.
    executable: PathBuf,
}

/// Why the installation could not be found.
#[derive(Debug, thiserror::Error)]
pub enum DetectError {
    /// The process could not say where its executable is.
    #[error("the app could not find its own executable")]
    CurrentExe(#[source] std::io::Error),
    /// The executable is not inside a `.app` bundle on macOS.
    #[error("the app is not inside a .app bundle")]
    NoBundle,
    /// The executable has no parent directory or file name.
    #[error("the app's executable has no directory")]
    NoDirectory,
}

/// Why replacing the app failed. The app is left as it was.
#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    /// The running executable could not be renamed aside (Windows).
    #[error("the running app could not be moved aside")]
    MoveAside(#[source] std::io::Error),
    /// The staged build could not be made executable (Unix).
    #[error("the update could not be made executable")]
    Permissions(#[source] std::io::Error),
    /// The staged build could not be renamed into the app's place.
    #[error("the update could not be put in the app's place")]
    Replace(#[source] std::io::Error),
    /// Replacing a bundle is not built yet.
    #[error("updating a {0} is not built yet")]
    Unsupported(Packaging),
}

impl Installation {
    /// Describe a copy from its executable's path, and on Linux from
    /// `$APPIMAGE` (set by the `AppImage` runtime to the image's own path).
    ///
    /// # Errors
    /// [`DetectError`] when the path does not fit the packaging.
    pub fn detect(
        packaging: Packaging,
        current_exe: PathBuf,
        appimage: Option<PathBuf>,
    ) -> Result<Self, DetectError> {
        if current_exe.parent().is_none() || current_exe.file_name().is_none() {
            return Err(DetectError::NoDirectory);
        }
        match (packaging, appimage) {
            // An AppImage runs from a mount; the file to replace is the
            // image the runtime names, wherever the owner put it.
            (Packaging::Executable | Packaging::AppImage, Some(image)) => {
                if image.parent().is_none() || image.file_name().is_none() {
                    return Err(DetectError::NoDirectory);
                }
                Ok(Self {
                    packaging: Packaging::AppImage,
                    executable: image.clone(),
                    path: image,
                })
            }
            (Packaging::Executable | Packaging::AppImage, None) => Ok(Self {
                packaging: Packaging::Executable,
                executable: current_exe.clone(),
                path: current_exe,
            }),
            (Packaging::AppBundle, _) => {
                let bundle = current_exe
                    .ancestors()
                    .find(|ancestor| ancestor.extension().is_some_and(|ext| ext == "app"))
                    .ok_or(DetectError::NoBundle)?
                    .to_path_buf();
                Ok(Self {
                    packaging: Packaging::AppBundle,
                    path: bundle,
                    executable: current_exe,
                })
            }
        }
    }

    /// Describe the running copy: its executable, and `$APPIMAGE`.
    ///
    /// # Errors
    /// [`DetectError`] as [`Self::detect`].
    pub fn from_process(default: Packaging) -> Result<Self, DetectError> {
        let current_exe = std::env::current_exe().map_err(DetectError::CurrentExe)?;
        let appimage = if cfg!(target_os = "linux") {
            std::env::var_os("APPIMAGE").map(PathBuf::from)
        } else {
            None
        };
        Self::detect(default, current_exe, appimage)
    }

    /// How this copy is packaged, which picks its artifact in the feed.
    #[must_use]
    pub const fn packaging(&self) -> Packaging {
        self.packaging
    }

    /// The executable to start after an update.
    #[must_use]
    pub fn executable(&self) -> &Path {
        &self.executable
    }

    /// The directory an update is staged in: the one holding what it
    /// replaces, so the swap is a rename on one volume.
    #[must_use]
    pub fn staging_dir(&self) -> &Path {
        // `detect` refused a path without a parent.
        self.path.parent().unwrap_or(&self.path)
    }

    /// The name the replaced file is moved aside to on Windows.
    #[cfg(windows)]
    fn aside(&self, running: &Label) -> PathBuf {
        let mut name = self.file_name();
        name.push(format!(".old-{running}"));
        self.path.with_file_name(name)
    }

    fn file_name(&self) -> OsString {
        self.path
            .file_name()
            .map(OsString::from)
            .unwrap_or_default()
    }

    /// Put `staged` in this copy's place. `running` is the build now
    /// running, used to name what is moved aside.
    ///
    /// Blocking file I/O: call it from a blocking thread.
    ///
    /// # Errors
    /// [`InstallError`]; the copy on disk is then the one that was there.
    pub fn replace_with(&self, staged: &Staged, running: &Label) -> Result<(), InstallError> {
        match self.packaging {
            Packaging::Executable | Packaging::AppImage => {
                self.replace_file(staged.path(), running)
            }
            Packaging::AppBundle => Err(InstallError::Unsupported(Packaging::AppBundle)),
        }
    }

    #[cfg(windows)]
    fn replace_file(&self, staged: &Path, running: &Label) -> Result<(), InstallError> {
        let aside = self.aside(running);
        std::fs::rename(&self.path, &aside).map_err(InstallError::MoveAside)?;
        if let Err(error) = std::fs::rename(staged, &self.path) {
            // Put the running app back where it was; if even that fails,
            // the old build is still beside it under `aside`, and the
            // error says the update did not happen.
            if let Err(back) = std::fs::rename(&aside, &self.path) {
                tracing::error!(error = %back, "the app could not be put back after a failed update");
            }
            return Err(InstallError::Replace(error));
        }
        Ok(())
    }

    #[cfg(unix)]
    fn replace_file(&self, staged: &Path, _running: &Label) -> Result<(), InstallError> {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(staged, std::fs::Permissions::from_mode(0o755))
            .map_err(InstallError::Permissions)?;
        std::fs::rename(staged, &self.path).map_err(InstallError::Replace)
    }

    /// Remove what earlier updates left beside the app: downloads cut
    /// short, and on Windows the builds moved aside. A file still in use
    /// (the build moved aside while it runs) stays until a later start.
    ///
    /// Blocking file I/O: call it from a blocking thread.
    pub fn clean_leftovers(&self) {
        let Ok(entries) = std::fs::read_dir(self.staging_dir()) else {
            return;
        };
        let mut aside_prefix = self.file_name();
        aside_prefix.push(".old-");
        let aside_prefix = aside_prefix.to_string_lossy().into_owned();
        for entry in entries.take(MAX_CLEAN_SCAN).flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let ours = name.starts_with(STAGING_PREFIX) || name.starts_with(&aside_prefix);
            let is_file = entry.file_type().is_ok_and(|kind| kind.is_file());
            if ours
                && is_file
                && let Err(error) = std::fs::remove_file(entry.path())
            {
                tracing::debug!(file = %name, error = %error, "an update leftover stays for now");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(text: &str) -> Label {
        text.parse().unwrap()
    }

    #[test]
    fn a_copy_is_found_from_its_executable_or_its_appimage() {
        let exe = PathBuf::from("/opt/vessel/vessel-app");
        let plain = Installation::detect(Packaging::Executable, exe.clone(), None).unwrap();
        assert_eq!(plain.packaging(), Packaging::Executable);
        assert_eq!(plain.executable(), exe);
        assert_eq!(plain.staging_dir(), Path::new("/opt/vessel"));

        let image = PathBuf::from("/home/someone/Apps/Vessel.AppImage");
        let mounted = PathBuf::from("/tmp/.mount_Vessel/usr/bin/vessel-app");
        let appimage =
            Installation::detect(Packaging::Executable, mounted, Some(image.clone())).unwrap();
        assert_eq!(appimage.packaging(), Packaging::AppImage);
        assert_eq!(appimage.executable(), image);
        assert_eq!(appimage.staging_dir(), Path::new("/home/someone/Apps"));

        let inside = PathBuf::from("/Applications/Vessel.app/Contents/MacOS/vessel-app");
        let bundle = Installation::detect(Packaging::AppBundle, inside.clone(), None).unwrap();
        assert_eq!(bundle.packaging(), Packaging::AppBundle);
        assert_eq!(bundle.executable(), inside);
        assert_eq!(bundle.staging_dir(), Path::new("/Applications"));

        assert!(matches!(
            Installation::detect(Packaging::AppBundle, exe, None),
            Err(DetectError::NoBundle)
        ));
        assert!(matches!(
            Installation::detect(Packaging::Executable, PathBuf::from("/"), None),
            Err(DetectError::NoDirectory)
        ));
    }

    #[test]
    fn the_staged_build_takes_the_apps_place() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("vessel-app");
        std::fs::write(&app, b"old build").unwrap();
        let installation = Installation::detect(Packaging::Executable, app.clone(), None).unwrap();
        let staged_path = dir.path().join(format!("{STAGING_PREFIX}0.0.0.0.2.1.part"));
        std::fs::write(&staged_path, b"new build").unwrap();
        let staged = Staged::for_test(staged_path.clone(), label("0.0.0.0.2"));

        installation
            .replace_with(&staged, &label("0.0.0.0.1"))
            .unwrap();

        assert_eq!(std::fs::read(&app).unwrap(), b"new build");
        assert!(!staged_path.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(&app).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o755);
        }
    }

    #[test]
    fn a_failed_swap_leaves_the_app_as_it_was() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("vessel-app");
        std::fs::write(&app, b"old build").unwrap();
        let installation = Installation::detect(Packaging::Executable, app.clone(), None).unwrap();
        let missing = Staged::for_test(dir.path().join("not-there"), label("0.0.0.0.2"));

        let result = installation.replace_with(&missing, &label("0.0.0.0.1"));

        assert!(result.is_err(), "{result:?}");
        assert_eq!(std::fs::read(&app).unwrap(), b"old build");
    }

    #[test]
    fn leftovers_are_removed_and_nothing_else() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("vessel-app.exe");
        let keep = [
            "vessel-app.exe",
            "notes.txt",
            "other.exe.old-0.0.0.0.1",
            "vessel-app.exe.older",
        ];
        let remove = [
            ".vessel-update-0.0.0.0.2.77.part",
            "vessel-app.exe.old-0.0.0.0.1",
        ];
        for name in keep.iter().chain(&remove) {
            std::fs::write(dir.path().join(name), b"x").unwrap();
        }
        std::fs::create_dir(dir.path().join(".vessel-update-dir")).unwrap();
        let installation = Installation::detect(Packaging::Executable, app, None).unwrap();

        installation.clean_leftovers();

        for name in keep {
            assert!(dir.path().join(name).exists(), "{name} was removed");
        }
        for name in remove {
            assert!(!dir.path().join(name).exists(), "{name} was left");
        }
        assert!(
            dir.path().join(".vessel-update-dir").exists(),
            "a directory was removed"
        );
    }

    #[test]
    fn a_bundle_is_not_replaced_yet() {
        let dir = tempfile::tempdir().unwrap();
        let inside = dir.path().join("Vessel.app/Contents/MacOS/vessel-app");
        let installation = Installation::detect(Packaging::AppBundle, inside, None).unwrap();
        let staged = Staged::for_test(dir.path().join("x"), label("0.0.0.0.2"));
        assert!(matches!(
            installation.replace_with(&staged, &label("0.0.0.0.1")),
            Err(InstallError::Unsupported(Packaging::AppBundle))
        ));
    }
}
