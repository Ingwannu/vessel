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
//!
//! [Decision Log] The set (ADR 0094)
//! - Purpose: the command line and the node ship beside the app; an
//!   update must leave the three of one build together, never a new app
//!   beside an old node or the reverse half way.
//! - Constraints: the swap above, per file; the node may be running
//!   (the app's managed local node), which Windows allows to be renamed
//!   but not replaced; no helper process.
//! - Alternatives: an archive unpacked over the folder (a crash half way
//!   leaves a mix, and nothing to go back to); a directory per build with
//!   a pointer to the current one (a launcher or a link the installer, the
//!   Start menu and the portable zip would all have to follow).
//! - Choice: every file of the set is downloaded and verified first
//!   ([`crate::stage`]); then each is put in place in turn, the companions
//!   first and the app last, keeping what it replaced aside: on Unix a
//!   hard link to the old file before the staged one is renamed over it
//!   (the path is never missing), on Windows the rename aside the app
//!   already used. When any file fails, every file already placed is put
//!   back (the aside renamed over the new one, or a file the set added
//!   removed), in reverse order. When all are placed, Unix removes the
//!   asides; Windows leaves them for the next start's clean-up, as before.
//! - Why: one code path for the app alone and for the set; nothing to
//!   launch; the folder holds either the old set or the new one after any
//!   single failure.
//! - Cost: a crash of the app between two renames of the set leaves a
//!   mix until the next check (which repairs the companions of the running
//!   label); on Unix an aside link needs a file system with hard links
//!   (every one a home directory sits on), and without them the update
//!   fails before changing anything.

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::{
    label::Label,
    manifest::{COMPANION_DIR, Role},
    platform::{Packaging, Target},
    stage::STAGING_PREFIX,
    stage::Staged,
};

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

/// A companion downloaded for the set, with the file name it takes in the
/// app's [`COMPANION_DIR`] ([`Role::file_name`]).
#[derive(Debug)]
pub(crate) struct StagedCompanion {
    /// The file it becomes, in the app's [`COMPANION_DIR`].
    pub(crate) file_name: &'static str,
    /// The verified download.
    pub(crate) staged: Staged,
}

/// One file of a set put in place, and where the file it replaced is
/// kept until the whole set is in.
#[derive(Debug)]
struct Placed {
    dest: PathBuf,
    /// `None`: the set added this file (there was none before).
    aside: Option<PathBuf>,
}

/// Why replacing the app failed. The app is left as it was.
#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    /// The file being replaced could not be moved aside (renamed on
    /// Windows, linked on Unix).
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

    /// Where a companion of `file_name` sits: in [`COMPANION_DIR`] beside
    /// the app's executable.
    #[must_use]
    pub fn companion_path(&self, file_name: &str) -> PathBuf {
        self.companion_dir().join(file_name)
    }

    /// The app's [`COMPANION_DIR`]: on the app's volume, so a companion's
    /// swap is a rename like the app's.
    fn companion_dir(&self) -> PathBuf {
        self.staging_dir().join(COMPANION_DIR)
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
        self.replace_set(Some(staged), &[], running)
    }

    /// Put a set in place (ADR 0094): `companions` beside the app, then
    /// `app` in the app's place; `app` is `None` when only companions are
    /// missing beside the running build. All or nothing: when one file
    /// fails, every file already placed is put back.
    ///
    /// Blocking file I/O: call it from a blocking thread.
    ///
    /// # Errors
    /// [`InstallError`]; the files on disk are then the ones that were
    /// there (unless putting one back failed too, which is logged and
    /// leaves the old file beside it under its aside name).
    pub(crate) fn replace_set(
        &self,
        app: Option<&Staged>,
        companions: &[StagedCompanion],
        running: &Label,
    ) -> Result<(), InstallError> {
        match self.packaging {
            Packaging::Executable => {}
            // An AppImage carries everything inside its one file: there is
            // nothing beside it to keep in step.
            Packaging::AppImage if companions.is_empty() => {}
            packaging @ (Packaging::AppImage | Packaging::AppBundle) => {
                return Err(InstallError::Unsupported(packaging));
            }
        }
        let swaps = companions
            .iter()
            .map(|companion| {
                (
                    self.companion_path(companion.file_name),
                    companion.staged.path(),
                )
            })
            .chain(app.map(|staged| (self.path.clone(), staged.path())));
        let mut placed: Vec<Placed> = Vec::new();
        for (dest, staged) in swaps {
            match place(dest, staged, running) {
                Ok(done) => placed.push(done),
                Err(error) => {
                    roll_back(&placed);
                    return Err(error);
                }
            }
        }
        // The whole set is in. On Unix the old files are links nothing
        // needs (a running process keeps its own inode); on Windows they
        // may still run, so the next start removes them.
        #[cfg(unix)]
        for done in &placed {
            if let Some(aside) = &done.aside
                && let Err(error) = std::fs::remove_file(aside)
            {
                tracing::warn!(
                    aside = %crate::service::file_name(aside),
                    error = %crate::report::chain(&error),
                    "a replaced file stays for the next start's clean-up"
                );
            }
        }
        #[cfg(windows)]
        if let Some(staged) = app {
            record_version(&self.path, staged.label());
        }
        if app.is_some() {
            tracing::info!(
                executable = %crate::service::file_name(&self.path),
                companions = companions.len(),
                "the new build is in place"
            );
        } else {
            tracing::info!(
                companions = companions.len(),
                "the running build's companions are in place"
            );
        }
        Ok(())
    }

    /// Remove what earlier updates left beside the app: on Windows the
    /// builds moved aside (the app's and its companions'). A file still in
    /// use (a build moved aside while it runs) stays until a later start.
    /// Downloads cut short stay: the next attempt goes on from them, and
    /// [`Self::prune_staging`] removes the ones a manifest no longer wants.
    ///
    /// Blocking file I/O: call it from a blocking thread.
    pub fn clean_leftovers(&self) {
        let mut aside_prefix = self.file_name();
        aside_prefix.push(".old-");
        let app_prefixes = vec![aside_prefix.to_string_lossy().into_owned()];
        clean_dir(self.staging_dir(), &app_prefixes);
        if let Some(target) = Target::current() {
            let companion_prefixes: Vec<String> = Role::KNOWN
                .iter()
                .filter_map(|role| role.file_name(target))
                .map(|name| format!("{name}.old-"))
                .collect();
            clean_dir(&self.companion_dir(), &companion_prefixes);
        }
    }
}

impl Installation {
    /// Remove every staging file beside the app but those named in `keep`
    /// (the files of the set the feed offers now): downloads of a build the
    /// feed has moved past, and of the processes before resuming existed.
    ///
    /// Blocking file I/O: call it from a blocking thread.
    pub(crate) fn prune_staging(&self, keep: &[String]) {
        let Ok(entries) = std::fs::read_dir(self.staging_dir()) else {
            return;
        };
        for entry in entries.take(MAX_CLEAN_SCAN).flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let is_file = entry.file_type().is_ok_and(|kind| kind.is_file());
            if !is_file
                || !name.starts_with(STAGING_PREFIX)
                || keep.iter().any(|kept| *kept == name)
            {
                continue;
            }
            match std::fs::remove_file(entry.path()) {
                Ok(()) => {
                    tracing::info!(file = %name, "a download the feed no longer offers is removed");
                }
                Err(error) => tracing::warn!(
                    file = %name,
                    error = %crate::report::chain(&error),
                    "a staging file stays for now"
                ),
            }
        }
    }
}

/// Remove the files in `dir` whose names start with one of `prefixes`
/// (directories never), looking at most at [`MAX_CLEAN_SCAN`] entries.
fn clean_dir(dir: &Path, prefixes: &[String]) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.take(MAX_CLEAN_SCAN).flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let ours = prefixes
            .iter()
            .any(|prefix| name.starts_with(prefix.as_str()));
        let is_file = entry.file_type().is_ok_and(|kind| kind.is_file());
        if !(ours && is_file) {
            continue;
        }
        match std::fs::remove_file(entry.path()) {
            Ok(()) => tracing::info!(file = %name, "an update leftover is removed"),
            Err(error) => tracing::warn!(
                file = %name,
                error = %crate::report::chain(&error),
                "an update leftover stays for now"
            ),
        }
    }
}

/// The name a replaced file is kept under until its set is whole:
/// `<name>.old-<running>`, the name the Windows swap always used.
fn aside_of(dest: &Path, running: &Label) -> PathBuf {
    let mut name = dest.file_name().map(OsString::from).unwrap_or_default();
    name.push(format!(".old-{running}"));
    dest.with_file_name(name)
}

/// Put the verified file `staged` at `dest`, keeping what was there aside.
fn place(dest: PathBuf, staged: &Path, running: &Label) -> Result<Placed, InstallError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(staged, std::fs::Permissions::from_mode(0o755))
            .map_err(InstallError::Permissions)?;
    }
    // A copy installed before the set has no companion folder yet.
    if let Some(parent) = dest.parent()
        && !parent.is_dir()
    {
        std::fs::create_dir_all(parent).map_err(InstallError::Replace)?;
    }
    let aside = if std::fs::symlink_metadata(&dest).is_ok() {
        let aside = aside_of(&dest, running);
        // A leftover under the same name from an earlier attempt; if it
        // cannot go (a Windows build still running), keeping it aside
        // below fails and the update stops before changing anything.
        let _gone = std::fs::remove_file(&aside).is_ok();
        // Unix: a second name for the old file, so `dest` is never missing
        // and the old file can be renamed back over the new one. Windows:
        // the running executable may only be renamed.
        #[cfg(unix)]
        std::fs::hard_link(&dest, &aside).map_err(InstallError::MoveAside)?;
        #[cfg(windows)]
        std::fs::rename(&dest, &aside).map_err(InstallError::MoveAside)?;
        Some(aside)
    } else {
        None
    };
    if let Err(error) = std::fs::rename(staged, &dest) {
        // Nothing replaced `dest`. Unix: it is still the old file, so only
        // the second name goes (renaming a link over its own file would do
        // nothing and leave it). Windows: the old file comes back from
        // aside.
        #[cfg(unix)]
        if let Some(aside) = &aside {
            let _gone = std::fs::remove_file(aside).is_ok();
        }
        #[cfg(windows)]
        put_back(&Placed { dest, aside });
        return Err(InstallError::Replace(error));
    }
    Ok(Placed { dest, aside })
}

/// Undo [`place`] for one file: the old file back in its place, or the
/// added file removed.
fn put_back(placed: &Placed) {
    let result = match &placed.aside {
        Some(aside) => std::fs::rename(aside, &placed.dest),
        None => match std::fs::remove_file(&placed.dest) {
            // The rename that failed never created it.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            other => other,
        },
    };
    match result {
        Ok(()) => tracing::info!(
            file = %crate::service::file_name(&placed.dest),
            "the file is as it was before the update"
        ),
        Err(error) => tracing::error!(
            file = %crate::service::file_name(&placed.dest),
            error = %crate::report::chain(&error),
            "a file could not be put back after a failed update"
        ),
    }
}

/// Undo every file of a set placed so far, the last first.
fn roll_back(placed: &[Placed]) {
    for done in placed.iter().rev() {
        put_back(done);
    }
}

/// Tell the installer's "Installed apps" entry the new version, when the
/// copy at `exe` is the installed one (`registry`, Windows only).
#[cfg(windows)]
fn record_version(exe: &Path, label: &Label) {
    match crate::registry::record_version(crate::registry::UNINSTALL_KEY, exe, label) {
        Ok(true) => tracing::info!("the installed app's entry shows the new version"),
        Ok(false) => {}
        Err(error) => tracing::warn!(
            error = %crate::report::chain(&error),
            "the installed app's entry keeps its old version"
        ),
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
        // A download cut short is kept at start: the next attempt goes on
        // from it (the manifest read prunes it if it is no longer wanted).
        let keep = [
            "vessel-app.exe",
            "notes.txt",
            "other.exe.old-0.0.0.0.1",
            "vessel-app.exe.older",
            ".vessel-update-0.0.0.0.2.77.part",
        ];
        let remove = ["vessel-app.exe.old-0.0.0.0.1"];
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
    fn pruning_keeps_only_the_staging_files_the_feed_still_wants() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("vessel-app");
        let wanted = ".vessel-update-0.0.0.0.20.0123456789abcdef.gz.part";
        let names = [
            "vessel-app",
            wanted,
            ".vessel-update-0.0.0.0.19.fedcba9876543210.part",
            ".vessel-update-0.0.0.0.2.77.part",
        ];
        for name in names {
            std::fs::write(dir.path().join(name), b"x").unwrap();
        }
        std::fs::create_dir(dir.path().join(".vessel-update-dir")).unwrap();
        let installation = Installation::detect(Packaging::Executable, app, None).unwrap();

        installation.prune_staging(&[wanted.to_owned()]);

        assert_eq!(
            names_of(dir.path()),
            [
                ".vessel-update-0.0.0.0.20.0123456789abcdef.gz.part",
                ".vessel-update-dir",
                "vessel-app"
            ]
        );
    }

    fn names_of(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    /// The names in `dir`, sorted.
    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_companions_aside_is_a_leftover_too() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join(COMPANION_DIR);
        std::fs::create_dir(&bin).unwrap();
        let node = Role::Node.file_name(Target::current().unwrap()).unwrap();
        let aside = format!("{node}.old-0.0.0.0.19");
        std::fs::write(dir.path().join("vessel-app"), b"x").unwrap();
        for name in [node, aside.as_str()] {
            std::fs::write(bin.join(name), b"x").unwrap();
        }
        let installation =
            Installation::detect(Packaging::Executable, dir.path().join("vessel-app"), None)
                .unwrap();

        installation.clean_leftovers();

        assert_eq!(names(&bin), [node], "{aside} was left or the node removed");
    }

    /// A staged companion of `file_name` holding `bytes`, staged beside
    /// the app as the service stages it.
    fn staged_companion(dir: &Path, file_name: &'static str, bytes: &[u8]) -> StagedCompanion {
        let path = dir.join(format!("{STAGING_PREFIX}0.0.0.0.20.1.{file_name}.part"));
        std::fs::write(&path, bytes).unwrap();
        StagedCompanion {
            file_name,
            staged: Staged::for_test(path, label("0.0.0.0.20")),
        }
    }

    #[test]
    fn a_set_takes_the_place_of_the_old_files_together() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join(COMPANION_DIR);
        std::fs::create_dir(&bin).unwrap();
        let app = dir.path().join("vessel-app");
        std::fs::write(&app, b"old app").unwrap();
        std::fs::write(bin.join("vessel"), b"old cli").unwrap();
        let installation = Installation::detect(Packaging::Executable, app.clone(), None).unwrap();
        let app_staged = dir
            .path()
            .join(format!("{STAGING_PREFIX}0.0.0.0.20.1.part"));
        std::fs::write(&app_staged, b"new app").unwrap();
        let companions = [
            staged_companion(dir.path(), "vessel", b"new cli"),
            staged_companion(dir.path(), "vesseld", b"new node"),
        ];

        installation
            .replace_set(
                Some(&Staged::for_test(app_staged, label("0.0.0.0.20"))),
                &companions,
                &label("0.0.0.0.19"),
            )
            .unwrap();

        assert_eq!(std::fs::read(&app).unwrap(), b"new app");
        assert_eq!(std::fs::read(bin.join("vessel")).unwrap(), b"new cli");
        assert_eq!(std::fs::read(bin.join("vesseld")).unwrap(), b"new node");
        #[cfg(unix)]
        {
            assert_eq!(names(dir.path()), ["bin", "vessel-app"]);
            assert_eq!(names(&bin), ["vessel", "vesseld"]);
        }
    }

    #[test]
    fn the_companion_folder_is_made_when_a_copy_has_none() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("vessel-app");
        std::fs::write(&app, b"app").unwrap();
        let installation = Installation::detect(Packaging::Executable, app, None).unwrap();
        let companions = [staged_companion(dir.path(), "vesseld", b"node")];

        installation
            .replace_set(None, &companions, &label("0.0.0.0.20"))
            .unwrap();

        assert_eq!(
            std::fs::read(installation.companion_path("vesseld")).unwrap(),
            b"node"
        );
        assert_eq!(names(dir.path()), ["bin", "vessel-app"]);
    }

    #[test]
    fn a_failure_half_way_puts_every_placed_file_back() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join(COMPANION_DIR);
        std::fs::create_dir(&bin).unwrap();
        let app = dir.path().join("vessel-app");
        std::fs::write(&app, b"old app").unwrap();
        std::fs::write(bin.join("vessel"), b"old cli").unwrap();
        let installation = Installation::detect(Packaging::Executable, app.clone(), None).unwrap();
        // Both companions go in; the app's staged file is gone, so the
        // last step of the set fails.
        let companions = [
            staged_companion(dir.path(), "vessel", b"new cli"),
            staged_companion(dir.path(), "vesseld", b"new node"),
        ];
        let missing = Staged::for_test(dir.path().join("not-there"), label("0.0.0.0.20"));

        let result = installation.replace_set(Some(&missing), &companions, &label("0.0.0.0.19"));

        // Unix stops at making the missing file executable, Windows at
        // its rename; either way the set fails and is put back.
        assert!(
            matches!(
                result,
                Err(InstallError::Permissions(_) | InstallError::Replace(_))
            ),
            "{result:?}"
        );
        assert_eq!(std::fs::read(&app).unwrap(), b"old app");
        assert_eq!(
            std::fs::read(bin.join("vessel")).unwrap(),
            b"old cli",
            "the command line is put back"
        );
        assert_eq!(
            names(&bin),
            ["vessel"],
            "the node the set added, or an aside, remains"
        );
        assert_eq!(
            names(dir.path()),
            ["bin", "vessel-app"],
            "a staged file remains"
        );
    }

    #[test]
    fn companions_beside_an_appimage_or_a_bundle_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let image = dir.path().join("Vessel.AppImage");
        let installation = Installation::detect(
            Packaging::Executable,
            dir.path().join("mount/vessel-app"),
            Some(image),
        )
        .unwrap();
        let companions = [staged_companion(dir.path(), "vesseld", b"node")];
        assert!(matches!(
            installation.replace_set(None, &companions, &label("0.0.0.0.19")),
            Err(InstallError::Unsupported(Packaging::AppImage))
        ));
        assert!(!dir.path().join(COMPANION_DIR).exists());
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
