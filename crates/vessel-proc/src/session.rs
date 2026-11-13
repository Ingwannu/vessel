//! [`kill_session`]: end every process left in a terminal's session.
//!
//! [Decision Log]
//! - Purpose: a shell in a PTY leads its own session (`setsid`), and an
//!   interactive shell puts every job (`npm run dev &`) in a process group
//!   of its own, inside that session. A signal to the shell's group misses
//!   them, so a closed terminal could leave a dev server running, holding
//!   its port, with nothing in Vessel that knows of it (ADR 0031).
//! - Choice: once the shell has exited, kill every process whose session
//!   id is the shell's pid. Linux lists every process's session in
//!   `/proc/<pid>/stat`; the system does not hand a pid out again while a
//!   session or a group still uses it as its id, so the sweep reaches only
//!   the shell's leftovers (the race the group sweep in this crate accepts
//!   remains: the session empties and a new process takes the id within
//!   microseconds).
//! - Elsewhere: macOS has no unprivileged list of a session's members
//!   other than running `ps`, and the shell's own `SIGHUP` to its jobs
//!   (bash and zsh pass it on when the terminal closes) is what ends them;
//!   Windows has no sessions of this kind (the pseudo console's closing
//!   ends what is attached to it). Both answer 0.
//! - A process that left the session on purpose (`setsid`, a daemon) is
//!   not touched: it asked not to belong to the terminal.

/// Kill (`SIGKILL`) every process whose session is `session` (a pid that
/// led one); the number killed. Blocking: reads `/proc`.
///
/// # Errors
/// `/proc` cannot be listed.
#[cfg(target_os = "linux")]
pub fn kill_session(session: u32) -> std::io::Result<usize> {
    use nix::sys::signal::{Signal as NixSignal, kill};
    use nix::unistd::Pid;

    // Never the node's own session, nor init's: a stray id must not take
    // the machine down.
    let own = nix::unistd::getsid(None)
        .ok()
        .and_then(|sid| u32::try_from(sid.as_raw()).ok());
    if session <= 1 || own == Some(session) {
        return Ok(0);
    }
    let mut killed = 0_usize;
    for entry in std::fs::read_dir("/proc")? {
        let Ok(entry) = entry else { continue };
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<i32>().ok())
        else {
            continue;
        };
        let Ok(stat) = std::fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        if stat_session(&stat) == Some(session)
            && kill(Pid::from_raw(pid), NixSignal::SIGKILL).is_ok()
        {
            killed = killed.saturating_add(1);
        }
    }
    Ok(killed)
}

/// See the Linux version: elsewhere nothing is found, and nothing killed.
///
/// # Errors
/// Never.
#[cfg(not(target_os = "linux"))]
pub const fn kill_session(_session: u32) -> std::io::Result<usize> {
    Ok(0)
}

/// The session id in a `/proc/<pid>/stat` line: the fourth field after the
/// command's closing parenthesis (state, parent, group, session).
#[cfg(any(target_os = "linux", test))]
fn stat_session(stat: &str) -> Option<u32> {
    let (_, rest) = stat.rsplit_once(')')?;
    rest.split_whitespace().nth(3)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_session_is_the_fourth_field_after_the_command() {
        assert_eq!(
            stat_session("269005 (a b) c) R 268468 269005 268400 0 -1"),
            Some(268_400)
        );
        assert_eq!(stat_session("nonsense"), None);
    }

    #[test]
    fn the_nodes_own_session_and_init_are_never_swept() {
        assert_eq!(kill_session(0).unwrap(), 0);
        assert_eq!(kill_session(1).unwrap(), 0);
    }
}
