//! The SSH steps against a "remote" that is a temporary home on this
//! machine. Unix only: the remote side of milestone 2 is (Windows remotes
//! are out of scope), `fake-ssh` needs `sh`, and the stand-in `vesseld`
//! and `gh` are shell scripts.
//!
//! `ssh` is `vessel-testkit`'s `fake-ssh`: it drops the options and runs
//! the command under `sh -c` in the fake home, with `HOME` set to it.

mod connector;
mod install;
mod release;
mod ssh;

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vessel_client::RemotePath;

use crate::ssh::Ssh;

/// A remote machine (an empty home directory), and a local directory for
/// builds, archives and stand-in tools.
pub(crate) struct Remote {
    pub(crate) home: TempDir,
    tools: TempDir,
}

impl Remote {
    pub(crate) fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let tools = tempfile::tempdir().unwrap();
        Self { home, tools }
    }

    pub(crate) fn ssh(&self) -> Ssh {
        Ssh::new("gpu-box".parse().unwrap())
            .with_program(vessel_testkit::Fake::Ssh.path().unwrap())
            .with_env("FAKE_SSH_HOME", self.home.path())
    }

    pub(crate) fn data_dir() -> RemotePath {
        RemotePath::default_data_dir()
    }

    /// `~/.vessel/bin` on the remote, seen from here.
    pub(crate) fn bin(&self) -> PathBuf {
        self.home.path().join(".vessel").join("bin")
    }

    /// The names in `~/.vessel/bin`, sorted.
    pub(crate) fn installed(&self) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(self.bin()) else {
            return Vec::new();
        };
        let mut names: Vec<_> = entries
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    /// A directory for local files (builds, archives).
    pub(crate) fn local(&self) -> &Path {
        self.tools.path()
    }
}

/// What a fake `vesseld` does when run on the remote (`FAKE_SSH_HOME` is
/// set there, not here).
#[derive(Debug, Clone, Copy)]
pub(crate) enum OnRemote {
    Works,
    /// Fails as a `-linux-gnu` build does on an older glibc.
    MissingGlibc,
    /// Reports another version.
    OtherVersion,
}

/// This machine's target triple, as a local build would report it.
pub(crate) fn local_triple() -> String {
    let os = if cfg!(target_os = "macos") {
        "apple-darwin"
    } else {
        "unknown-linux-gnu"
    };
    format!("{}-{os}", std::env::consts::ARCH)
}

/// Write a fake `vesseld` at `path` reporting `version` and `triple`.
pub(crate) fn fake_vesseld(path: &Path, version: &str, triple: &str, remote: OnRemote) {
    let on_remote = match remote {
        OnRemote::Works => "",
        OnRemote::MissingGlibc => {
            "echo \"vesseld: /lib/libc.so.6: version 'GLIBC_2.99' not found\" >&2; exit 1"
        }
        OnRemote::OtherVersion => "echo \"9.9.9 $TRIPLE\"; exit 0",
    };
    let script = format!(
        "#!/bin/sh\n\
         TRIPLE='{triple}'\n\
         if [ -n \"${{FAKE_SSH_HOME:-}}\" ]; then :; {on_remote}\nfi\n\
         case \"$1\" in\n\
         version) echo '{version} {triple}' ;;\n\
         cleanup) ;;\n\
         *) exit 2 ;;\n\
         esac\n"
    );
    executable(path, &script);
}

/// Write an executable script at `path` (never with `std::fs::write`; see
/// `vessel_testkit::write_executable`).
pub(crate) fn executable(path: &Path, text: &str) {
    vessel_testkit::write_executable(path, text).unwrap();
}

pub(crate) fn mode(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}
