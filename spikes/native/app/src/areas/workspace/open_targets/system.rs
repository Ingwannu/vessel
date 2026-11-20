//! This computer, for Open in editor (ADR 0123): what the rules read from
//! it ([`Computer`]) and how a target is started on it ([`System`]).
//!
//! [Decision Log]
//! - Purpose: start the editor the person chose with the path they chose,
//!   as Paseo's desktop does (`runtime.ts`), and nothing else.
//! - Constraint (the coordinator's ruling on ADR 0123): the program is the
//!   one resolved once for this launch from PATH or the known install
//!   paths; it gets argv, never a shell line; no standard streams; its own
//!   process group; the app's environment without Vessel's own control
//!   variables; reaped off the UI thread. The resolved path is logged, the
//!   environment never.
//! - Choice: `std::process::Command` with `Stdio::null()` for all three
//!   streams, `process_group(0)` on Unix (safe std API), every
//!   `VESSEL_APP_*` variable removed (`main.rs` reads them), and a named
//!   thread that waits on the child. On Windows a `.cmd`/`.bat` goes through
//!   std's batch-file handling (cmd.exe with std's quoting, which refuses an
//!   argument it cannot quote safely: that refusal is the toast). The file
//!   manager is GPUI's (`reveal_path`, `open_with_system`), not a process of
//!   ours.
//! - Cost: a reaper thread per editor started, gone when the editor's
//!   launcher exits (most CLIs hand off to a running editor at once).

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use vessel_app_kit::ui::App;
use vessel_app_view::open_targets::{Entry, EntryKind, Env, Platform, Probe};

use super::launcher::Launcher;

/// The prefix of the variables that steer this app (`main.rs`).
#[cfg_attr(test, expect(dead_code, reason = "tests never start a program"))]
const OWN_PREFIX: &str = "VESSEL_APP_";

/// This computer as the rules read it.
#[derive(Debug, Clone)]
pub(crate) struct Computer {
    platform: Platform,
    env: Env,
}

impl Computer {
    /// This computer, with the variables it has now.
    #[cfg_attr(
        test,
        expect(dead_code, reason = "tests read a computer of stub files")
    )]
    pub(crate) fn here() -> Self {
        let var = |name: &str| std::env::var(name).ok();
        Self {
            platform: Platform::current(),
            env: Env {
                path: var("PATH"),
                home: var("HOME"),
                local_app_data: var("LOCALAPPDATA"),
                program_files: var("ProgramFiles"),
            },
        }
    }

    /// This computer read with `env` (tests: a PATH of stub files).
    #[cfg(test)]
    pub(crate) const fn with(platform: Platform, env: Env) -> Self {
        Self { platform, env }
    }
}

impl Probe for Computer {
    fn platform(&self) -> Platform {
        self.platform
    }

    fn env(&self) -> &Env {
        &self.env
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn entries(&self, dir: &Path) -> io::Result<Vec<Entry>> {
        std::fs::read_dir(dir)?
            .map(|entry| {
                let entry = entry?;
                let kind = entry.file_type().map_or(EntryKind::Other, |kind| {
                    if kind.is_dir() {
                        EntryKind::Directory
                    } else if kind.is_file() {
                        EntryKind::File
                    } else {
                        EntryKind::Other
                    }
                });
                Ok(Entry {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    kind,
                })
            })
            .collect()
    }

    fn canonical(&self, path: &Path) -> io::Result<PathBuf> {
        std::fs::canonicalize(path)
    }
}

/// The app's launcher: real programs, GPUI's file manager.
#[derive(Debug, Default)]
#[cfg_attr(test, expect(dead_code, reason = "tests never start a program"))]
pub(crate) struct System;

impl Launcher for System {
    fn start(&self, program: &Path, args: &[OsString]) -> io::Result<()> {
        let mut command = Command::new(program);
        let _args = command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        for (name, _value) in std::env::vars_os() {
            if name.to_string_lossy().starts_with(OWN_PREFIX) {
                let _removed = command.env_remove(name);
            }
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt as _;
            let _grouped = command.process_group(0);
        }
        tracing::info!(program = %program.display(), "starting an editor");
        let mut child = command.spawn()?;
        let reaper = std::thread::Builder::new()
            .name("vessel-editor-reaper".to_owned())
            .spawn(move || {
                let _status = child.wait();
            });
        if let Err(error) = reaper {
            // The editor runs; only its exit status goes unread until the
            // app exits.
            tracing::warn!(%error, "no thread to reap an editor's launcher");
        }
        Ok(())
    }

    fn reveal(&self, path: &Path, cx: &App) {
        cx.reveal_path(path);
    }

    fn open_folder(&self, path: &Path, cx: &App) {
        cx.open_with_system(path);
    }
}
