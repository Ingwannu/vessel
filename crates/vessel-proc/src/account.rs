//! The login shell of the account this process runs as (ADR 0031): what a
//! terminal beside a session runs when `$SHELL` names none (a node started
//! by a service manager often has no `$SHELL`).

use std::path::PathBuf;

/// The shell the password database gives this process's account; `None`
/// on Windows, or when the database has no entry or an empty shell.
#[must_use]
pub fn account_shell() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        let user = nix::unistd::User::from_uid(nix::unistd::getuid()).ok()??;
        Some(user.shell).filter(|shell| !shell.as_os_str().is_empty())
    }
    #[cfg(not(unix))]
    {
        None
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// Whatever the machine says, an answer is an absolute path.
    #[test]
    fn an_account_shell_is_absolute() {
        if let Some(shell) = account_shell() {
            assert!(shell.is_absolute(), "{}", shell.display());
        }
    }
}
