//! ADR 0111's report sections (TRACKER 567): the machine, its disk, the
//! workspaces, what runs and two tools, collected under the report's one
//! deadline beside ADR 0074's sources.
//!
//! [Decision Log]
//! - Purpose: Paseo's System, Disk, Workspaces, Agents and Tools sections,
//!   as numbers and closed states.
//! - Sources: the standard library for processors; Linux's `/proc/loadavg`
//!   and `/proc/meminfo`, read with a byte cap as `/proc/self/status` is
//!   (other systems answer "unsupported" rather than run a command);
//!   `statvfs` through the store (`DataDir::space`); the workspace
//!   registry; the runtime's live sessions; `git --version` and
//!   `gh --version`.
//! - The two tools are the only commands a report runs (superseding ADR
//!   0074's "no `--version`" for them): fixed programs found on the node's
//!   search path, no arguments from anywhere, [`TOOL_TIMEOUT`] each, the
//!   first [`TOOL_OUTPUT_LIMIT`] bytes read, and only three numbers kept.

use std::io::Read as _;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::AsyncReadExt as _;
use tokio::time::{Instant, timeout_at};
use vessel_proto::diagnostics::{
    DiagnosticCount, DiagnosticDisk, DiagnosticLoad, DiagnosticMore, DiagnosticSection,
    DiagnosticSystem, DiagnosticSystemMemory, DiagnosticTool, DiagnosticTools,
    DiagnosticUnavailable, DiagnosticVersion, DiagnosticWorkspaces,
};
use vessel_proto::registry::{WorkspaceBacking, WorkspaceLifecycle};

use super::State;
use super::admission::Lease;
use super::collect::{available, unavailable};

/// How long one tool's `--version` may take (Paseo's 3 s).
const TOOL_TIMEOUT: Duration = Duration::from_secs(3);
/// How much of its answer is read (Paseo's 512 bytes).
const TOOL_OUTPUT_LIMIT: u64 = 512;
/// How much of a `/proc` file is read.
const PROC_LIMIT: u64 = 16 * 1024;

impl State {
    /// The sections, each unavailable for its own reason when its source
    /// fails or the deadline passes.
    pub(super) async fn more(&self, lease: Arc<Lease>, deadline: Instant) -> DiagnosticMore {
        let dir = self.data_dir().clone();
        let blocking = tokio::task::spawn_blocking(move || {
            let _retained = lease;
            (system(), disk(&dir))
        });
        let lookup = self.lookup.clone();
        let (blocking, workspaces, tools) = tokio::join!(
            timeout_at(deadline, blocking),
            timeout_at(deadline, self.store.workspace_registry()),
            timeout_at(deadline, tools(&lookup)),
        );
        let (system, disk) = match blocking {
            Ok(Ok(sections)) => sections,
            Ok(Err(_join)) => (
                unavailable(DiagnosticUnavailable::ReadFailed),
                unavailable(DiagnosticUnavailable::ReadFailed),
            ),
            Err(_deadline) => (
                unavailable(DiagnosticUnavailable::TimedOut),
                unavailable(DiagnosticUnavailable::TimedOut),
            ),
        };
        let workspaces = match workspaces {
            Ok(Ok(registry)) => available(workspace_counts(&registry.workspaces)),
            Ok(Err(_store)) => unavailable(DiagnosticUnavailable::ReadFailed),
            Err(_deadline) => unavailable(DiagnosticUnavailable::TimedOut),
        };
        DiagnosticMore {
            system,
            disk,
            workspaces,
            activity: available(self.runtime.diagnostic_activity()),
            tools: tools.map_or_else(
                |_deadline| unavailable(DiagnosticUnavailable::TimedOut),
                available,
            ),
        }
    }
}

fn system() -> DiagnosticSection<DiagnosticSystem> {
    let cpus = std::thread::available_parallelism()
        .map_or(0, |count| u64::try_from(count.get()).unwrap_or(u64::MAX));
    available(DiagnosticSystem {
        cpus: DiagnosticCount::new(cpus),
        load: proc_file("/proc/loadavg").as_deref().and_then(load),
        memory: proc_file("/proc/meminfo").as_deref().and_then(memory),
    })
}

/// A `/proc` file's first [`PROC_LIMIT`] bytes, on Linux only.
fn proc_file(path: &str) -> Option<String> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let mut text = String::new();
    let _read = std::fs::File::open(path)
        .ok()?
        .take(PROC_LIMIT)
        .read_to_string(&mut text)
        .ok()?;
    Some(text)
}

/// `/proc/loadavg`'s first three fields, in hundredths.
pub(super) fn load(text: &str) -> Option<DiagnosticLoad> {
    let mut fields = text.split_whitespace().map(hundredths);
    Some(DiagnosticLoad {
        one: fields.next()??,
        five: fields.next()??,
        fifteen: fields.next()??,
    })
}

/// `0.42` as 42: digits only, two decimals kept.
fn hundredths(text: &str) -> Option<u32> {
    let (whole, part) = text.split_once('.').unwrap_or((text, ""));
    let whole: u32 = whole.parse().ok()?;
    let mut digits = part.chars().chain(std::iter::repeat('0')).take(2);
    let tens = digits.next()?.to_digit(10)?;
    let ones = digits.next()?.to_digit(10)?;
    whole
        .checked_mul(100)?
        .checked_add(tens.checked_mul(10)?)?
        .checked_add(ones)
}

/// `/proc/meminfo`'s `MemAvailable` and `MemTotal`, in bytes.
pub(super) fn memory(text: &str) -> Option<DiagnosticSystemMemory> {
    let kib = |name: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(name)?.strip_prefix(':'))
            .and_then(|rest| rest.split_whitespace().next()?.parse::<u64>().ok())
            .map(|value| DiagnosticCount::new(value.saturating_mul(1024)))
    };
    Some(DiagnosticSystemMemory {
        available_bytes: kib("MemAvailable")?,
        total_bytes: kib("MemTotal")?,
    })
}

fn disk(dir: &vessel_store::DataDir) -> DiagnosticSection<DiagnosticDisk> {
    match dir.space() {
        Ok(space) => available(DiagnosticDisk {
            available_bytes: DiagnosticCount::new(space.available),
            total_bytes: DiagnosticCount::new(space.total),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::Unsupported => {
            unavailable(DiagnosticUnavailable::Unsupported)
        }
        Err(_other) => unavailable(DiagnosticUnavailable::ReadFailed),
    }
}

fn workspace_counts(
    workspaces: &[vessel_proto::registry::WorkspaceDescriptor],
) -> DiagnosticWorkspaces {
    let one = DiagnosticCount::new(1);
    let mut counts = DiagnosticWorkspaces {
        projects: DiagnosticCount::default(),
        active: DiagnosticCount::default(),
        managed: DiagnosticCount::default(),
        archived: DiagnosticCount::default(),
    };
    let mut projects = std::collections::HashSet::new();
    for workspace in workspaces {
        let _new = projects.insert(workspace.project);
        match workspace.lifecycle {
            WorkspaceLifecycle::Active => {
                counts.active = counts.active.saturating_add(one);
                if matches!(workspace.backing, WorkspaceBacking::Managed { .. }) {
                    counts.managed = counts.managed.saturating_add(one);
                }
            }
            // Being archived or restored: not active until it completes.
            WorkspaceLifecycle::Archiving { .. }
            | WorkspaceLifecycle::Restoring { .. }
            | WorkspaceLifecycle::Archived { .. } => {
                counts.archived = counts.archived.saturating_add(one);
            }
        }
    }
    counts.projects = DiagnosticCount::new(u64::try_from(projects.len()).unwrap_or(u64::MAX));
    counts
}

async fn tools(lookup: &vessel_harness::Lookup) -> DiagnosticTools {
    // Finding a program reads the file system: a blocking task's work.
    let search = lookup.clone();
    let found = tokio::task::spawn_blocking(move || (search.find("git"), search.find("gh"))).await;
    let Ok((git, gh)) = found else {
        return DiagnosticTools {
            git: DiagnosticTool::Failed,
            gh: DiagnosticTool::Failed,
        };
    };
    let (git, gh) = tokio::join!(tool(lookup, git), tool(lookup, gh));
    DiagnosticTools { git, gh }
}

/// `<path> --version`, for a program found on the node's search path.
async fn tool(lookup: &vessel_harness::Lookup, path: Option<std::path::PathBuf>) -> DiagnosticTool {
    let Some(path) = path else {
        return DiagnosticTool::Missing;
    };
    let mut command = tokio::process::Command::new(path);
    let _command = command
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    if let Some(search) = lookup.search_path() {
        let _env = command.env("PATH", search);
    }
    let Ok(mut child) = command.spawn() else {
        return DiagnosticTool::Failed;
    };
    let read = async {
        let mut text = String::new();
        if let Some(stdout) = child.stdout.take() {
            let _read = stdout
                .take(TOOL_OUTPUT_LIMIT)
                .read_to_string(&mut text)
                .await
                .ok()?;
        }
        let status = child.wait().await.ok()?;
        status.success().then_some(text)
    };
    match tokio::time::timeout(TOOL_TIMEOUT, read).await {
        Ok(Some(text)) => DiagnosticTool::Found {
            version: version_in(&text),
        },
        Ok(None) => DiagnosticTool::Failed,
        Err(_elapsed) => DiagnosticTool::TimedOut,
    }
}

/// The first word of `text` that reads as a three-number version
/// (`git version 2.43.0`, `gh version 2.40.1 (2024-01-01)`).
pub(super) fn version_in(text: &str) -> Option<DiagnosticVersion> {
    text.split_whitespace()
        .find_map(|word| word.trim_start_matches('v').parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_files_and_tool_answers_read_as_numbers_only() {
        let loads = load("0.42 1.05 12.7 3/456 7890\n").unwrap();
        assert_eq!((loads.one, loads.five, loads.fifteen), (42, 105, 1270));
        assert_eq!(load("0.42 x 1.0"), None);
        assert_eq!(load(""), None);
        let meminfo = "MemTotal:       16384 kB\nMemFree:  10 kB\nMemAvailable:    8192 kB\n";
        let memory = memory(meminfo).unwrap();
        assert_eq!(memory.total_bytes.get(), 16384 * 1024);
        assert_eq!(memory.available_bytes.get(), 8192 * 1024);
        assert_eq!(super::memory("MemTotal: 1 kB\n"), None);
        let git = version_in("git version 2.43.0\n").unwrap();
        assert_eq!((git.major, git.minor, git.patch), (2, 43, 0));
        let gh = version_in("gh version 2.40.1 (2024-01-01)\nhttps://example.invalid\n").unwrap();
        assert_eq!((gh.major, gh.minor, gh.patch), (2, 40, 1));
        assert_eq!(version_in("git version 2.39 (Apple Git-146)"), None);
    }
}
