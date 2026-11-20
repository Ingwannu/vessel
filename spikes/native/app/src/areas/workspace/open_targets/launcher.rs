//! The one seam through which Open in editor reaches outside the app
//! (ADR 0123): a program started, an entry revealed, a folder opened.
//! The app's own is `system::System`; tests use `Recording`, so no
//! test starts an editor or a file manager.

use std::ffi::OsString;
use std::io;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

use vessel_app_kit::ui::App;

/// What Open in editor starts.
pub(crate) trait Launcher: Send + Sync {
    /// Start `program` with `args`, detached; the UI never waits on it.
    ///
    /// # Errors
    ///
    /// When it cannot be started.
    fn start(&self, program: &Path, args: &[OsString]) -> io::Result<()>;
    /// Show `path` selected in the file manager.
    fn reveal(&self, path: &Path, cx: &App);
    /// Open the folder `path` in the file manager.
    fn open_folder(&self, path: &Path, cx: &App);
}

/// A launcher that starts nothing: in tests before one is set, and on the
/// page's build, where no machine is this computer.
#[cfg(any(test, target_arch = "wasm32"))]
#[derive(Debug, Default)]
pub(crate) struct Nothing;

#[cfg(any(test, target_arch = "wasm32"))]
impl Launcher for Nothing {
    fn start(&self, _program: &Path, _args: &[OsString]) -> io::Result<()> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }

    fn reveal(&self, _path: &Path, _cx: &App) {}

    fn open_folder(&self, _path: &Path, _cx: &App) {}
}

/// What a test's launcher was asked to do.
#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Launched {
    /// A program and its arguments.
    Start(PathBuf, Vec<OsString>),
    /// A reveal.
    Reveal(PathBuf),
    /// A folder opened.
    OpenFolder(PathBuf),
}

/// A launcher that only writes down what it was asked.
#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct Recording(pub(crate) std::sync::Mutex<Vec<Launched>>);

#[cfg(test)]
impl Recording {
    /// Everything asked so far.
    pub(crate) fn asked(&self) -> Vec<Launched> {
        self.0.lock().map(|asked| asked.clone()).unwrap_or_default()
    }

    fn note(&self, launched: Launched) {
        if let Ok(mut asked) = self.0.lock() {
            asked.push(launched);
        }
    }
}

#[cfg(test)]
impl Launcher for Recording {
    fn start(&self, program: &Path, args: &[OsString]) -> io::Result<()> {
        self.note(Launched::Start(program.to_path_buf(), args.to_vec()));
        Ok(())
    }

    fn reveal(&self, path: &Path, _cx: &App) {
        self.note(Launched::Reveal(path.to_path_buf()));
    }

    fn open_folder(&self, path: &Path, _cx: &App) {
        self.note(Launched::OpenFolder(path.to_path_buf()));
    }
}
