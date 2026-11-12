//! Linux: everything from `/proc`, nothing run. Blocking reads, on the
//! blocking pool.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::parse;
use super::{Listener, Scan, ScanError, ScanLimits, Trees};

/// How many processes of the whole system are read for their parents:
/// far above any real machine's count, so a runaway `/proc` cannot hold
/// the scan.
const MAX_SYSTEM_PROCESSES: usize = 262_144;

pub(super) async fn scan(roots: &[u32], limits: ScanLimits) -> Result<Scan, ScanError> {
    let roots = roots.to_vec();
    tokio::task::spawn_blocking(move || scan_proc(Path::new("/proc"), &roots, limits))
        .await
        .map_err(|stopped| ScanError::Read {
            source_name: "/proc",
            error: std::io::Error::other(stopped),
        })?
}

/// The scan, over a `/proc` at `proc` (a folder of the tests' own in
/// tests).
pub(super) fn scan_proc(proc: &Path, roots: &[u32], limits: ScanLimits) -> Result<Scan, ScanError> {
    let read = |error| ScanError::Read {
        source_name: "/proc",
        error,
    };
    let mut pairs = Vec::new();
    let mut truncated = false;
    for entry in std::fs::read_dir(proc).map_err(read)? {
        let Ok(entry) = entry else { continue };
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        if pairs.len() >= MAX_SYSTEM_PROCESSES {
            truncated = true;
            break;
        }
        // A process that exits between the listing and the read is gone.
        if let Ok(stat) = std::fs::read_to_string(entry.path().join("stat"))
            && let Some(parent) = parse::proc_stat_parent(&stat)
        {
            pairs.push((pid, parent));
        }
    }
    let trees = Trees::build(roots, pairs, limits.processes);
    truncated |= trees.truncated();

    let (sockets, descriptors_truncated) = socket_owners(proc, &trees, limits.descriptors);
    truncated |= descriptors_truncated;

    let mut listeners = Vec::new();
    let mut seen = HashSet::new();
    let mut names: HashMap<u32, Option<String>> = HashMap::new();
    for (file, v6) in [("net/tcp", false), ("net/tcp6", true)] {
        // A kernel without IPv6 has no `tcp6`.
        let Ok(table) = std::fs::read_to_string(proc.join(file)) else {
            continue;
        };
        for socket in parse::proc_net_tcp(&table, v6) {
            let Some(&pid) = sockets.get(&socket.inode) else {
                continue;
            };
            let Some(root) = trees.root_of(pid) else {
                continue;
            };
            if !seen.insert((pid, socket.address, socket.port)) {
                continue;
            }
            let process = names
                .entry(pid)
                .or_insert_with(|| {
                    std::fs::read_to_string(proc.join(pid.to_string()).join("comm"))
                        .ok()
                        .map(|name| name.trim().to_owned())
                })
                .clone();
            listeners.push(Listener {
                root,
                pid,
                process,
                address: socket.address,
                port: socket.port,
            });
        }
    }
    Ok(Scan {
        listeners,
        truncated,
    })
}

/// The socket inodes the trees' processes hold, each with its process;
/// and whether the descriptor bound stopped the walk.
fn socket_owners(proc: &Path, trees: &Trees, limit: usize) -> (HashMap<u64, u32>, bool) {
    let mut owners = HashMap::new();
    let mut read = 0_usize;
    for pid in trees.processes() {
        let Ok(descriptors) = std::fs::read_dir(proc.join(pid.to_string()).join("fd")) else {
            continue;
        };
        for descriptor in descriptors {
            if read >= limit {
                return (owners, true);
            }
            read = read.saturating_add(1);
            let Ok(descriptor) = descriptor else { continue };
            let Ok(target) = std::fs::read_link(descriptor.path()) else {
                continue;
            };
            let Some(inode) = target
                .to_str()
                .and_then(|target| target.strip_prefix("socket:["))
                .and_then(|rest| rest.strip_suffix(']'))
                .and_then(|inode| inode.parse::<u64>().ok())
            else {
                continue;
            };
            let _first = owners.entry(inode).or_insert(pid);
        }
    }
    (owners, false)
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, TcpListener};

    use super::*;

    /// The test's own process listens on loopback; a scan rooted at it
    /// finds that port, and one rooted elsewhere does not.
    #[test]
    fn finds_its_own_listener_in_the_real_proc() {
        let socket = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = socket.local_addr().unwrap().port();
        let me = std::process::id();
        let scan = scan_proc(Path::new("/proc"), &[me], ScanLimits::DEFAULT).unwrap();
        let found = scan
            .listeners
            .iter()
            .find(|listener| listener.port == port)
            .unwrap_or_else(|| panic!("port {port} not found in {:?}", scan.listeners));
        assert_eq!(found.root, me);
        assert_eq!(found.pid, me);
        assert_eq!(found.address, IpAddr::V4(Ipv4Addr::LOCALHOST));

        // A tree rooted at a process that does not exist holds nothing
        // but its root.
        let elsewhere =
            scan_proc(Path::new("/proc"), &[u32::MAX - 1], ScanLimits::DEFAULT).unwrap();
        assert!(
            elsewhere
                .listeners
                .iter()
                .all(|listener| listener.port != port)
        );
        drop(socket);
    }

    #[test]
    fn a_missing_proc_is_an_error() {
        let empty = tempfile_dir();
        assert!(scan_proc(&empty.join("none"), &[1], ScanLimits::DEFAULT).is_err());
    }

    fn tempfile_dir() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("vessel-proc-listening-{}", std::process::id()))
    }
}
