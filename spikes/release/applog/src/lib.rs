//! The app's log: what it says goes to stderr, as before, and to a file
//! the owner can find and send, kept to a bounded size, with any panic in
//! it.
//!
//! [Decision Log]
//! - Purpose: the first update on the owner's Windows (0.0.0.0.3) failed
//!   and nothing said why. A Windows GUI process started from the Start
//!   menu has no stderr, and stderr was the app's only log.
//! - Constraints: the log is the owner's to send, so it must not hold
//!   secrets (the updater never logs the feed's path, tested there) and
//!   must not grow without bound; logging must never stop the app; the
//!   app's code is the desktop track's, so this is one call for `main`
//!   and one function for its About screen.
//! - Alternatives: `tracing-appender` (rotates by time only: a day of
//!   heavy logging is unbounded; a non-blocking writer loses the last
//!   lines when a panic aborts); the platform's own logs (the Windows
//!   event log needs a registered source and an installer step, and the
//!   owner cannot just send it); a log only when asked (`RUST_LOG` and a
//!   redirected stderr: the failure has already happened by then).
//! - Choice: a file in the platform's place for logs ([`dir`]), written
//!   as each line comes (blocking, so a panic's lines are on disk before
//!   the process ends), rotated by size: past [`MAX_BYTES`] the file
//!   becomes `vessel.1.log`, that one `vessel.2.log`, and the oldest goes;
//!   so at most three files, about 3 MiB. `RUST_LOG` sets the filter as
//!   before; by default everything at info and the updater at debug. A
//!   panic hook writes the message, the place and a backtrace, then lets
//!   the previous hook run.
//! - Cost: a write per line on the thread that logs (the app logs little;
//!   the updater a few lines per check). If the file cannot be opened the
//!   app runs with stderr only and says so there.

use std::{
    fs::{File, OpenOptions},
    io::Write as _,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use tracing_subscriber::{
    EnvFilter, fmt::MakeWriter, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

/// The current file's name; older ones are `vessel.1.log`, `vessel.2.log`.
pub const FILE: &str = "vessel.log";

/// The size past which the current file is rotated.
pub const MAX_BYTES: u64 = 1024 * 1024;

/// How many older files are kept beside the current one.
pub const KEEP: usize = 2;

/// The filter when `RUST_LOG` is not set.
pub const DEFAULT_FILTER: &str = "info,vessel_app_updater=debug";

/// Where the app's log files are, for the About screen's "open the log
/// folder" (`cx.open_with_system(&dir)` in GPUI):
///
/// - Windows: `%LOCALAPPDATA%\Vessel\logs`
/// - macOS: `~/Library/Logs/Vessel`
/// - Linux and others: `$XDG_STATE_HOME/vessel/logs`
///   (`~/.local/state/vessel/logs`)
///
/// `None` when the platform names no such directory.
#[must_use]
pub fn dir() -> Option<PathBuf> {
    if cfg!(windows) {
        dirs::data_local_dir().map(|base| base.join("Vessel").join("logs"))
    } else if cfg!(target_os = "macos") {
        dirs::home_dir().map(|home| home.join("Library").join("Logs").join("Vessel"))
    } else {
        dirs::state_dir()
            .or_else(dirs::data_local_dir)
            .map(|base| base.join("vessel").join("logs"))
    }
}

/// What [`init`] set up.
#[derive(Debug)]
pub struct Logging {
    file: Option<PathBuf>,
}

impl Logging {
    /// The file being written, when there is one.
    #[must_use]
    pub fn file(&self) -> Option<&Path> {
        self.file.as_deref()
    }
}

/// Log to stderr and to the file in [`dir`], and record panics there.
/// Call once, first thing in `main`; `label` is the build's.
#[must_use]
pub fn init(label: &str) -> Logging {
    init_in(dir().as_deref(), label)
}

/// [`init`] with the log in `dir` (tests; `None`: stderr only).
#[must_use]
pub fn init_in(dir: Option<&Path>, label: &str) -> Logging {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
    let opened = dir.map(|dir| Rotating::open(dir, MAX_BYTES, KEEP));
    let (file_layer, file, failure) = match opened {
        Some(Ok(rotating)) => {
            let path = rotating.current();
            let layer = tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .fmt_fields(PlainFields::default())
                .with_writer(FileLog(Arc::new(rotating)));
            (Some(layer), Some(path), None)
        }
        Some(Err(error)) => (None, None, Some(error)),
        None => (None, None, None),
    };
    let stderr = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);
    let installed = tracing_subscriber::registry()
        .with(filter)
        .with(stderr)
        .with(file_layer)
        .try_init();
    if installed.is_ok() {
        record_panics();
    }
    if let Some(error) = failure {
        tracing::warn!(%error, "the log file could not be opened; logging to stderr only");
    }
    tracing::info!(
        label,
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        "Vessel starts"
    );
    Logging { file }
}

/// Write each panic (message, place, backtrace) to the log, then run the
/// hook that was there before (Rust's prints to stderr).
fn record_panics() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        let message = info.payload_as_str().unwrap_or("(not a string)");
        let place = info
            .location()
            .map(|at| format!("{}:{}:{}", at.file(), at.line(), at.column()));
        #[cfg(windows)]
        let frames = frame_addresses();
        #[cfg(not(windows))]
        let frames = String::new();
        tracing::error!(message, place, frames, %backtrace, "the app panicked");
        previous(info);
    }));
}

/// The stack as addresses the build's kept `.pdb` turns into names: a
/// frame in the app is `exe+0x<rva>`, an address relative to the image's
/// base (`llvm-symbolizer --obj=<exe> --relative-address 0x<rva>`, with
/// the label's `vessel_app.pdb` beside the exe); a frame in another
/// module (a system DLL) is its address and that module's base. Return
/// addresses less one, so a call's line rather than the next one's.
///
/// Windows only: there std's backtrace prints `<unknown>` for every frame
/// without the `.pdb` (0.0.0.0.6 under wine), and the `.pdb` is kept
/// beside the label under the build directory, not shipped
/// (`build.sh`).
#[cfg(windows)]
fn frame_addresses() -> String {
    const MAX_FRAMES: usize = 64;
    let mut ours = None;
    let mut frames = Vec::new();
    backtrace::trace(|frame| {
        // The first frame is this crate's code, linked into the exe: its
        // module is the app's.
        let ip = frame.ip().addr();
        let base = frame.module_base_address().map(<*mut _>::addr);
        let first = frames.is_empty();
        let ip = if first { ip } else { ip.saturating_sub(1) };
        let ours_base = *ours.get_or_insert(base);
        frames.push(match base {
            Some(base) if Some(base) == ours_base => format!("exe+{:#x}", ip.saturating_sub(base)),
            Some(base) => format!("{ip:#x}@{base:#x}"),
            None => format!("{ip:#x}"),
        });
        frames.len() < MAX_FRAMES
    });
    frames.join(" ")
}

/// The file layer's field formatter: `DefaultFields` under a type of its
/// own. `tracing-subscriber` keeps a span's formatted fields in the span,
/// keyed by the formatter's type, and the layer that sees the span first
/// formats them; with one type for both layers the file got stderr's
/// colour codes (seen in 0.0.0.0.6's `vessel.log`). Its own type makes
/// the file layer format them itself, without colour.
#[derive(Debug, Default)]
struct PlainFields(tracing_subscriber::fmt::format::DefaultFields);

impl<'writer> tracing_subscriber::fmt::FormatFields<'writer> for PlainFields {
    fn format_fields<R: tracing_subscriber::field::RecordFields>(
        &self,
        writer: tracing_subscriber::fmt::format::Writer<'writer>,
        fields: R,
    ) -> std::fmt::Result {
        self.0.format_fields(writer, fields)
    }
}

/// The file layer's writer.
struct FileLog(Arc<Rotating>);

impl<'a> MakeWriter<'a> for FileLog {
    type Writer = Line<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        Line(&self.0)
    }
}

/// One formatted line, written whole (`tracing`'s fmt layer writes each
/// event with one `write_all`, so a rotation never splits a line).
struct Line<'a>(&'a Rotating);

impl std::io::Write for Line<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.write(bytes)?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A log file rotated by size.
#[derive(Debug)]
pub struct Rotating {
    dir: PathBuf,
    max: u64,
    keep: usize,
    open: Mutex<Open>,
}

#[derive(Debug)]
struct Open {
    /// `None` between closing the file for a rotation and reopening it
    /// (and after a reopen failed: the next line tries again).
    file: Option<File>,
    written: u64,
}

impl Rotating {
    /// Open (or create) `dir/vessel.log` for appending, rotating it first
    /// when it is already past `max`.
    ///
    /// # Errors
    /// The I/O error when the directory or the file cannot be made.
    pub fn open(dir: &Path, max: u64, keep: usize) -> std::io::Result<Self> {
        create_dir(dir)?;
        let current = dir.join(FILE);
        if std::fs::metadata(&current).is_ok_and(|meta| meta.len() >= max) {
            rotate(dir, keep);
        }
        let file = append(&current)?;
        let written = file.metadata().map_or(0, |meta| meta.len());
        Ok(Self {
            dir: dir.to_path_buf(),
            max,
            keep,
            open: Mutex::new(Open {
                file: Some(file),
                written,
            }),
        })
    }

    /// The current file's path.
    #[must_use]
    pub fn current(&self) -> PathBuf {
        self.dir.join(FILE)
    }

    /// Append `bytes`, rotating first when they would take the file past
    /// its size (a file never holds less than one line).
    ///
    /// # Errors
    /// The I/O error of the write; a failed rotation keeps writing to the
    /// current file.
    pub fn write(&self, bytes: &[u8]) -> std::io::Result<()> {
        let mut open = self
            .open
            .lock()
            .map_err(|_poisoned| std::io::Error::other("the log's lock is poisoned"))?;
        let len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if open.written > 0 && open.written.saturating_add(len) > self.max {
            // Closed before the rename, so the rename does not depend on
            // how Windows shares an open file.
            open.file = None;
            rotate(&self.dir, self.keep);
        }
        let file = if let Some(file) = open.file.take() {
            file
        } else {
            let file = append(&self.current())?;
            open.written = file.metadata().map_or(0, |meta| meta.len());
            file
        };
        let file = open.file.insert(file);
        file.write_all(bytes)?;
        open.written = open.written.saturating_add(len);
        Ok(())
    }
}

/// The log's folder (and any missing parent), owner-only where it is
/// made: the log names machines and folders, and the app's settings are
/// owner-only too (0700; the coordinator's check of 0.0.0.0.10 found the
/// folder 0775). Made with its mode rather than changed afterwards, so it
/// is never open in between; a folder that already exists is left as it
/// is.
fn create_dir(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt as _;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(dir)
    }
}

/// The current log file for appending, owner-only (0600) when it is made,
/// as the settings file is (it was 0664).
fn append(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    let _same = options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        let _same = options.mode(0o600);
    }
    options.open(path)
}

/// `vessel.log` → `vessel.1.log` → … → `vessel.<keep>.log`, the oldest
/// replaced. Failures leave files where they are.
fn rotate(dir: &Path, keep: usize) {
    let numbered = |n: usize| dir.join(format!("vessel.{n}.log"));
    for n in (1..keep).rev() {
        let _moved = std::fs::rename(numbered(n), numbered(n.saturating_add(1)));
    }
    if keep > 0 {
        let _moved = std::fs::rename(dir.join(FILE), numbered(1));
    } else {
        let _removed = std::fs::remove_file(dir.join(FILE));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// The folder and every file are owner-only from the moment they are
    /// made, the rotated one's successor too, whatever the umask leaves.
    #[cfg(unix)]
    #[test]
    fn the_log_folder_and_its_files_are_made_owner_only() {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("state").join("logs");
        let log = Rotating::open(&dir, 100, 2).unwrap();
        for n in 0..5 {
            log.write(format!("line {n:02} of forty bytes, give or take..\n").as_bytes())
                .unwrap();
        }
        assert_eq!(mode(&dir), 0o700);
        assert_eq!(mode(&root.path().join("state")), 0o700);
        for name in names(&dir) {
            assert_eq!(mode(&dir.join(&name)), 0o600, "{name}");
        }
    }

    #[test]
    fn past_its_size_the_file_rotates_and_only_two_older_ones_stay() {
        let dir = tempfile::tempdir().unwrap();
        let log = Rotating::open(dir.path(), 100, 2).unwrap();
        for n in 0..20 {
            log.write(format!("line {n:02} of forty bytes, give or take..\n").as_bytes())
                .unwrap();
        }
        assert_eq!(
            names(dir.path()),
            ["vessel.1.log", "vessel.2.log", "vessel.log"]
        );
        let newest = std::fs::read_to_string(dir.path().join(FILE)).unwrap();
        assert!(newest.contains("line 19"), "{newest}");
        for name in names(dir.path()) {
            let size = std::fs::metadata(dir.path().join(&name)).unwrap().len();
            assert!(size <= 100, "{name} is {size} bytes");
        }
        let older = std::fs::read_to_string(dir.path().join("vessel.1.log")).unwrap();
        assert!(
            !older.contains("line 19") && older.contains("line 1"),
            "{older}"
        );
    }

    #[test]
    fn a_file_already_past_its_size_is_rotated_at_start() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FILE), vec![b'x'; 150]).unwrap();
        let log = Rotating::open(dir.path(), 100, 2).unwrap();
        log.write(b"fresh\n").unwrap();
        assert_eq!(std::fs::read(dir.path().join(FILE)).unwrap(), b"fresh\n");
        assert_eq!(
            std::fs::metadata(dir.path().join("vessel.1.log"))
                .unwrap()
                .len(),
            150
        );
    }
}
