//! What each platform's sources say, read into plain facts. Pure, so every
//! platform's reader is tested on every platform (with text recorded from
//! the real tools).

#[cfg(any(target_os = "linux", target_os = "macos", windows, test))]
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[cfg(any(target_os = "linux", test))]
/// A listening socket as `/proc/net/tcp{,6}` lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ProcSocket {
    pub(super) inode: u64,
    pub(super) address: IpAddr,
    pub(super) port: u16,
}

#[cfg(any(target_os = "linux", test))]
/// The listening sockets (`st` `0A`) of `/proc/net/tcp` (`v6: false`) or
/// `/proc/net/tcp6`. Lines that do not read are skipped.
pub(super) fn proc_net_tcp(text: &str, v6: bool) -> Vec<ProcSocket> {
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let local = *fields.get(1)?;
            let state = *fields.get(3)?;
            let inode = fields.get(9)?.parse::<u64>().ok()?;
            if state != "0A" || inode == 0 {
                return None;
            }
            let (address, port) = local.split_once(':')?;
            let port = u16::from_str_radix(port, 16).ok()?;
            let address = if v6 {
                IpAddr::V6(proc_v6(address)?)
            } else {
                IpAddr::V4(proc_v4(address)?)
            };
            Some(ProcSocket {
                inode,
                address,
                port,
            })
        })
        .collect()
}

#[cfg(any(target_os = "linux", test))]
/// An IPv4 address as the kernel prints it: one 32-bit word in the
/// machine's byte order, which is little-endian on every platform Linux
/// runs Vessel on (`x86_64`, `aarch64`).
fn proc_v4(hex: &str) -> Option<Ipv4Addr> {
    let word = u32::from_str_radix(hex, 16).ok()?;
    Some(Ipv4Addr::from(word.swap_bytes()))
}

#[cfg(any(target_os = "linux", test))]
/// An IPv6 address: four 32-bit words, each in the machine's byte order.
fn proc_v6(hex: &str) -> Option<Ipv6Addr> {
    if hex.len() != 32 {
        return None;
    }
    let mut bytes = [0_u8; 16];
    for (index, chunk) in bytes.chunks_mut(4).enumerate() {
        let start = index.checked_mul(8)?;
        let word = u32::from_str_radix(hex.get(start..start.checked_add(8)?)?, 16).ok()?;
        chunk.copy_from_slice(&word.swap_bytes().to_be_bytes());
    }
    Some(Ipv6Addr::from(bytes))
}

#[cfg(any(target_os = "linux", test))]
/// A process's parent from its `/proc/<pid>/stat`: the field after the
/// state, after the last `)` (a command may hold spaces and parentheses).
pub(super) fn proc_stat_parent(text: &str) -> Option<u32> {
    let (_, rest) = text.rsplit_once(')')?;
    let mut fields = rest.split_whitespace();
    let _state = fields.next()?;
    fields.next()?.parse().ok()
}

#[cfg(any(target_os = "macos", test))]
/// `ps -A -o pid=,ppid=`: pairs of a process and its parent.
pub(super) fn ps_parents(text: &str) -> Vec<(u32, u32)> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let pid = fields.next()?.parse().ok()?;
            let parent = fields.next()?.parse().ok()?;
            Some((pid, parent))
        })
        .collect()
}

#[cfg(any(target_os = "macos", windows, test))]
/// A listener as `lsof` or `netstat` reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ToolListener {
    pub(super) pid: u32,
    pub(super) process: Option<String>,
    pub(super) address: IpAddr,
    pub(super) port: u16,
}

#[cfg(any(target_os = "macos", test))]
/// `lsof -nP -iTCP -sTCP:LISTEN -F pctn`: a `p` line starts a process, `c`
/// names it, then each file has `f`, `t` (`IPv4` or `IPv6`) and `n`
/// (`127.0.0.1:3000`, `[::1]:3000`, `*:3000`).
pub(super) fn lsof(text: &str) -> Vec<ToolListener> {
    let mut found = Vec::new();
    let mut pid = None;
    let mut process = None;
    let mut v6 = false;
    for line in text.lines() {
        let Some(kind) = line.chars().next() else {
            continue;
        };
        let value = line.get(1..).unwrap_or_default();
        match kind {
            'p' => {
                pid = value.parse::<u32>().ok();
                process = None;
            }
            'c' => process = Some(value.to_owned()),
            't' => v6 = value == "IPv6",
            'n' => {
                if let (Some(pid), Some((address, port))) = (pid, endpoint(value, v6)) {
                    found.push(ToolListener {
                        pid,
                        process: process.clone(),
                        address,
                        port,
                    });
                }
            }
            _ => {}
        }
    }
    found
}

#[cfg(any(target_os = "macos", windows, test))]
/// `host:port` as `lsof` and `netstat` write it: `[v6]:port`, `*:port`
/// (every address of the family), `a.b.c.d:port`.
fn endpoint(text: &str, v6: bool) -> Option<(IpAddr, u16)> {
    let (host, port) = text.rsplit_once(':')?;
    let port = port.parse::<u16>().ok()?;
    let address = match host {
        "*" if v6 => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
        "*" => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        bracketed if bracketed.starts_with('[') => {
            let inner = bracketed.strip_prefix('[')?.strip_suffix(']')?;
            // A zone (`fe80::1%en0`) is not part of the address.
            let inner = inner
                .split_once('%')
                .map_or(inner, |(address, _zone)| address);
            IpAddr::V6(inner.parse().ok()?)
        }
        plain => plain.parse().ok()?,
    };
    Some((address, port))
}

#[cfg(any(windows, test))]
/// `netstat -ano -p TCP` (or `TCPv6`) on Windows: `TCP  <local>  <remote>
/// LISTENING  <pid>`. The state's word is English only when the system's
/// language is; a listener is told by its remote end (`0.0.0.0:0`,
/// `[::]:0`) instead, which every language prints the same.
pub(super) fn netstat(text: &str) -> Vec<ToolListener> {
    text.lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields
                .first()
                .is_none_or(|protocol| !protocol.eq_ignore_ascii_case("tcp"))
            {
                return None;
            }
            let local = *fields.get(1)?;
            let remote = *fields.get(2)?;
            let pid = fields.last()?.parse::<u32>().ok()?;
            if !remote.ends_with(":0") || fields.len() != 5 {
                return None;
            }
            let (address, port) = endpoint(local, local.starts_with('['))?;
            Some(ToolListener {
                pid,
                process: None,
                address,
                port,
            })
        })
        .collect()
}

#[cfg(any(windows, test))]
/// Lines of `<pid>\t<parent>\t<name>` (the `PowerShell` query of
/// `Win32_Process`).
pub(super) fn cim_processes(text: &str) -> Vec<(u32, u32, Option<String>)> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.trim_end_matches('\r').split('\t');
            let pid = fields.next()?.trim().parse().ok()?;
            let parent = fields.next()?.trim().parse().ok()?;
            let name = fields
                .next()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_owned);
            Some((pid, parent, name))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TCP: &str = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 0100007F:4963 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 119268734 1 0000000000000000 100 0 0 10 0
   1: 00000000:0BB8 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 4242 1 0000000000000000 100 0 0 10 0
   2: 0100007F:9C40 0100007F:4963 01 00000000:00000000 00:00000000 00000000  1000        0 777 1 0000000000000000 100 0 0 10 0
";

    const TCP6: &str = "  sl  local_address                         remote_address                        st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 00000000000000000000000000000000:0016 00000000000000000000000000000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 8446 1 0000000000000000 100 0 0 10 0
   1: 00000000000000000000000001000000:1F90 00000000000000000000000000000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 9001 1 0000000000000000 100 0 0 10 0
";

    #[test]
    fn proc_lists_only_listeners_with_their_addresses() {
        let found = proc_net_tcp(TCP, false);
        assert_eq!(
            found,
            [
                ProcSocket {
                    inode: 119_268_734,
                    address: IpAddr::V4(Ipv4Addr::LOCALHOST),
                    port: 0x4963
                },
                ProcSocket {
                    inode: 4242,
                    address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                    port: 3000
                },
            ]
        );
        let found = proc_net_tcp(TCP6, true);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].address, IpAddr::V6(Ipv6Addr::UNSPECIFIED));
        assert_eq!(found[0].port, 22);
        assert_eq!(found[1].address, IpAddr::V6(Ipv6Addr::LOCALHOST));
        assert_eq!(found[1].port, 8080);
    }

    #[test]
    fn a_parent_is_read_after_the_command() {
        assert_eq!(
            proc_stat_parent("269005 (cat) R 268468 269005 268468 0 -1"),
            Some(268_468)
        );
        assert_eq!(
            proc_stat_parent("42 (a (b) c) S 7 42 42 0 -1"),
            Some(7),
            "a command with spaces and parentheses"
        );
        assert_eq!(proc_stat_parent("garbage"), None);
    }

    #[test]
    fn ps_pairs() {
        assert_eq!(
            ps_parents("    1     0\n  412     1\nbad\n"),
            [(1, 0), (412, 1)]
        );
    }

    /// Recorded from `lsof -nP -iTCP -sTCP:LISTEN -a -p … -F pctn` on
    /// macOS (the process ids changed).
    const LSOF: &str = "p501\ncnode\nf23\ntIPv4\nn127.0.0.1:5173\nf24\ntIPv6\nn[::1]:5173\np777\ncpython3.12\nf3\ntIPv6\nn*:8000\nf4\ntIPv4\nn*:8001\n";

    #[test]
    fn lsof_files_belong_to_the_process_before_them() {
        let found = lsof(LSOF);
        let summary: Vec<(u32, Option<&str>, String, u16)> = found
            .iter()
            .map(|found| {
                (
                    found.pid,
                    found.process.as_deref(),
                    found.address.to_string(),
                    found.port,
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                (501, Some("node"), "127.0.0.1".to_owned(), 5173),
                (501, Some("node"), "::1".to_owned(), 5173),
                (777, Some("python3.12"), "::".to_owned(), 8000),
                (777, Some("python3.12"), "0.0.0.0".to_owned(), 8001),
            ]
        );
    }

    /// Recorded from `netstat -ano -p TCP` and `-p TCPv6` on Windows 11,
    /// one line in another language's state word.
    const NETSTAT: &str = "
Active Connections

  Proto  Local Address          Foreign Address        State           PID
  TCP    0.0.0.0:135            0.0.0.0:0              LISTENING       1032
  TCP    127.0.0.1:5173         0.0.0.0:0              ABHÖREN         8812
  TCP    127.0.0.1:5173         127.0.0.1:50211        ESTABLISHED     8812
  TCP    [::]:445               [::]:0                 LISTENING       4
  TCP    [::1]:3000             [::]:0                 LISTENING       9120
";

    #[test]
    fn netstat_listeners_whatever_the_language() {
        let found: Vec<(u32, String, u16)> = netstat(NETSTAT)
            .into_iter()
            .map(|found| (found.pid, found.address.to_string(), found.port))
            .collect();
        assert_eq!(
            found,
            [
                (1032, "0.0.0.0".to_owned(), 135),
                (8812, "127.0.0.1".to_owned(), 5173),
                (4, "::".to_owned(), 445),
                (9120, "::1".to_owned(), 3000),
            ]
        );
    }

    #[test]
    fn cim_lines() {
        assert_eq!(
            cim_processes("4\t0\tSystem\r\n8812\t7700\tnode.exe\r\n9\t1\t\r\nbad\r\n"),
            [
                (4, 0, Some("System".to_owned())),
                (8812, 7700, Some("node.exe".to_owned())),
                (9, 1, None),
            ]
        );
    }

    #[test]
    fn endpoints_with_zones() {
        assert_eq!(
            endpoint("[fe80::1%en0]:22", true),
            Some(("fe80::1".parse().unwrap(), 22))
        );
        assert_eq!(endpoint("nonsense", false), None);
    }
}
