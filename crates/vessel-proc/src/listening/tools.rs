//! macOS and Windows: the system's own tools, run as children of their own
//! group (or job), bounded in time and output.

use std::collections::HashSet;

use tokio::io::AsyncReadExt as _;

use super::parse::{self, ToolListener};
use super::{Listener, Scan, ScanError, ScanLimits, Trees};
use crate::{CommandSpec, Signal, Spawned, spawn};

pub(super) async fn scan(roots: &[u32], limits: ScanLimits) -> Result<Scan, ScanError> {
    let (trees, names) = processes(roots, limits).await?;
    let found = listeners(&trees, limits).await?;
    let mut seen = HashSet::new();
    let listeners = found
        .into_iter()
        .filter_map(|found| {
            let root = trees.root_of(found.pid)?;
            seen.insert((found.pid, found.address, found.port))
                .then(|| Listener {
                    root,
                    pid: found.pid,
                    process: found.process.or_else(|| {
                        names
                            .iter()
                            .find(|(pid, _)| *pid == found.pid)
                            .map(|(_, name)| name.clone())
                    }),
                    address: found.address,
                    port: found.port,
                })
        })
        .collect();
    Ok(Scan {
        listeners,
        truncated: trees.truncated(),
    })
}

/// The trees, and the processes' names where the tool gave them.
#[cfg(target_os = "macos")]
async fn processes(
    roots: &[u32],
    limits: ScanLimits,
) -> Result<(Trees, Vec<(u32, String)>), ScanError> {
    let text = run(
        "ps",
        CommandSpec::new("/bin/ps").args(["-A", "-o", "pid=,ppid="]),
        limits,
    )
    .await?;
    Ok((
        Trees::build(roots, parse::ps_parents(&text), limits.processes),
        Vec::new(),
    ))
}

#[cfg(windows)]
async fn processes(
    roots: &[u32],
    limits: ScanLimits,
) -> Result<(Trees, Vec<(u32, String)>), ScanError> {
    // One query, tab-separated so a name with spaces stays one field.
    const QUERY: &str = "Get-CimInstance Win32_Process | ForEach-Object { \"$($_.ProcessId)`t$($_.ParentProcessId)`t$($_.Name)\" }";
    let spec = CommandSpec::new("powershell.exe").args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        QUERY,
    ]);
    let text = run("powershell", spec, limits).await?;
    let rows = parse::cim_processes(&text);
    let trees = Trees::build(
        roots,
        rows.iter().map(|(pid, parent, _)| (*pid, *parent)),
        limits.processes,
    );
    let names = rows
        .into_iter()
        .filter(|(pid, _, _)| trees.root_of(*pid).is_some())
        .filter_map(|(pid, _, name)| name.map(|name| (pid, name)))
        .collect();
    Ok((trees, names))
}

/// The listeners of the trees' processes.
#[cfg(target_os = "macos")]
async fn listeners(trees: &Trees, limits: ScanLimits) -> Result<Vec<ToolListener>, ScanError> {
    if trees.is_empty() {
        return Ok(Vec::new());
    }
    let pids: Vec<String> = trees.processes().map(|pid| pid.to_string()).collect();
    let spec = CommandSpec::new("/usr/sbin/lsof")
        .args(["-nP", "-iTCP", "-sTCP:LISTEN", "-a", "-p"])
        .arg(pids.join(","))
        .args(["-F", "pctn"]);
    // `lsof` exits with 1 when it found nothing.
    let text = run_allowing("lsof", spec, limits, &[0, 1]).await?;
    Ok(parse::lsof(&text))
}

#[cfg(windows)]
async fn listeners(trees: &Trees, limits: ScanLimits) -> Result<Vec<ToolListener>, ScanError> {
    if trees.is_empty() {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    for family in ["TCP", "TCPv6"] {
        let spec = CommandSpec::new("netstat.exe").args(["-ano", "-p", family]);
        let text = run("netstat", spec, limits).await?;
        found.extend(parse::netstat(&text));
    }
    Ok(found)
}

/// Run `spec` to its end; its standard output as text, when it exits with
/// 0.
async fn run(
    tool: &'static str,
    spec: CommandSpec,
    limits: ScanLimits,
) -> Result<String, ScanError> {
    run_allowing(tool, spec, limits, &[0]).await
}

/// Run `spec` to its end, its output bounded and its time limited; its
/// standard output when it exits with one of `codes`.
async fn run_allowing(
    tool: &'static str,
    spec: CommandSpec,
    limits: ScanLimits,
    codes: &[i32],
) -> Result<String, ScanError> {
    let Spawned {
        mut child,
        stdin,
        stdout,
        stderr,
    } = spawn(&spec).map_err(|error| ScanError::Read {
        source_name: tool,
        error: std::io::Error::other(error),
    })?;
    drop(stdin);
    drop(stderr);
    let bound = u64::try_from(limits.tool_output).unwrap_or(u64::MAX);
    let mut bytes = Vec::new();
    let read = async {
        let mut limited = stdout.take(bound);
        let _read = limited.read_to_end(&mut bytes).await;
        child.wait().await
    };
    let code = match tokio::time::timeout(limits.tool_time, read).await {
        Ok(Ok(code)) => code,
        Ok(Err(_)) => None,
        Err(_elapsed) => {
            if let Err(error) = child.signal(Signal::Kill) {
                tracing::debug!(%error, tool, "cannot stop a scan's tool");
            }
            let _code = child.wait().await;
            return Err(ScanError::TimedOut { tool });
        }
    };
    if !code.is_some_and(|code| codes.contains(&code)) {
        return Err(ScanError::Failed { tool, code });
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
