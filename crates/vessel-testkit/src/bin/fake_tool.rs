//! `fake-tool`: a stand-in for a command-line tool `vessel init` runs (an
//! installer such as `npm`, a service manager such as `systemctl` or
//! `launchctl`), so its tests can see what ran without running anything.
//!
//! Behaviour:
//!
//! - Its name is `FAKE_TOOL_NAME` if set (a Windows `.cmd` shim sets it),
//!   else the name it was run as (a symbolic link's name).
//! - Each call appends one line to the file `FAKE_TOOL_LOG` names: the name,
//!   then the arguments, separated by single spaces. Without it (an
//!   installer the node runs in a clean environment, `agent.install`), the
//!   file is `fake-tool.log` in the home (`HOME`, `USERPROFILE`), and the
//!   line is also printed, as an installer prints its progress.
//! - Queries answer from that log, as a service manager would from its
//!   state: a call with the argument `is-enabled` succeeds only if an
//!   earlier call of the same tool had `enable`; `list` as the first
//!   argument succeeds only if an earlier one had `load`.
//! - `FAKE_TOOL_FAIL` set: every call fails (exit 1), after being logged.
//! - Anything else succeeds (exit 0) and prints nothing.

use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args();
    let program = args.next().unwrap_or_default();
    let name = std::env::var("FAKE_TOOL_NAME").unwrap_or_else(|_unset| {
        Path::new(&program)
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    let args: Vec<String> = args.collect();
    let (log, print) = if let Some(log) = std::env::var_os("FAKE_TOOL_LOG") {
        (std::path::PathBuf::from(log), false)
    } else {
        let home = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
        let Some(home) = std::env::var_os(home) else {
            return ExitCode::from(2);
        };
        (Path::new(&home).join("fake-tool.log"), true)
    };
    let earlier = std::fs::read_to_string(&log).unwrap_or_default();
    let line = std::iter::once(name.as_str())
        .chain(args.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ");
    let logged = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
        .and_then(|mut file| writeln!(file, "{line}"));
    if logged.is_err() {
        return ExitCode::from(2);
    }
    if print {
        let printed = writeln!(std::io::stdout().lock(), "fake-tool: {line}");
        drop(printed);
    }
    if std::env::var_os("FAKE_TOOL_FAIL").is_some() {
        return ExitCode::FAILURE;
    }
    let had = |word: &str| {
        earlier.lines().any(|line| {
            let mut words = line.split(' ');
            words.next() == Some(name.as_str()) && words.any(|earlier| earlier == word)
        })
    };
    let answer = if args.iter().any(|arg| arg == "is-enabled") {
        had("enable")
    } else if args.first().map(String::as_str) == Some("list") {
        had("load")
    } else {
        true
    };
    if answer {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
