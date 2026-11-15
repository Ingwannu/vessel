//! Linux candidate launch data. A matching executable is NOT engine qualification.
//!
//! The future supervisor owns this plan/profile through actual child cleanup;
//! the node actor may offer a browser only after the complete reviewed runtime,
//! proxy, input, Preview and platform qualification, not merely these hashes.
mod bundle;
mod identity;
#[cfg(test)]
mod tests;

use crate::profile::Profile;
use identity::{Candidate, LaunchError};
use std::ffi::OsString;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use vessel_proc::{Child, CommandSpec};

const FIXED_SHELL: &str = "/bin/sh";
const FIXED_SOURCE: &str = "umask 077; exec 3<&0 4>&1; exec \"$@\"";
const INERT_LABEL: &str = "vessel-browser";
/// One owner for the actual qualified flag combination; no caller adds flags.
const FLAGS: &[&str] = &[
    "--headless=new",
    "--remote-debugging-pipe",
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-background-networking",
    "--disable-component-update",
    "--disable-sync",
    "--disable-extensions",
    "--disable-quic",
    "--webrtc-ip-handling-policy=disable_non_proxied_udp",
    "--proxy-bypass-list=<-loopback>",
];

/// Constructed from the private policy proxy's listener, not a browser URL.
#[derive(Clone, Copy)]
pub(crate) struct ProxyEndpoint(SocketAddr);
impl std::fmt::Debug for ProxyEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BrowserProxyEndpoint([private])")
    }
}
impl TryFrom<SocketAddr> for ProxyEndpoint {
    type Error = LaunchError;
    fn try_from(address: SocketAddr) -> Result<Self, Self::Error> {
        if address.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST) || address.port() == 0 {
            Err(LaunchError::Proxy)
        } else {
            Ok(Self(address))
        }
    }
}
/// This remains unqualified launch data, never a ready-engine or permission token.
pub(crate) struct Plan {
    command: CommandSpec,
    profile: Profile,
    identity: Candidate,
}
impl std::fmt::Debug for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserCandidatePlan")
            .finish_non_exhaustive()
    }
}
impl Plan {
    /// Synchronous file hashing must run through the finite `Preparation` owner.
    pub(crate) fn inspect(profile: Profile, proxy: ProxyEndpoint) -> Result<Self, LaunchError> {
        let identity = Candidate::inspect()?;
        Ok(Self::build(profile, proxy, identity))
    }
    fn build(profile: Profile, proxy: ProxyEndpoint, identity: Candidate) -> Self {
        let mut directory = OsString::from("--user-data-dir=");
        directory.push(profile.root());
        let command = CommandSpec::new(FIXED_SHELL)
            .env_clear()
            .cwd(profile.root())
            .args(["-c", FIXED_SOURCE, INERT_LABEL, Candidate::browser_path()])
            .args(FLAGS.iter().copied())
            .arg(format!("--proxy-server=http://{}", proxy.0))
            .arg(directory)
            .arg("about:blank")
            .env("HOME", profile.home())
            .env("XDG_CONFIG_HOME", profile.config())
            .env("XDG_CACHE_HOME", profile.cache())
            .env("TMPDIR", profile.temporary())
            .env("LANG", "C.UTF-8")
            .env("TZ", "UTC")
            .env("PATH", "/usr/bin:/bin");
        Self {
            command,
            profile,
            identity,
        }
    }
    /// The actual owned child, not a stored/caller PID, supplies identity.
    pub(crate) fn verify_started(&self, child: &Child) -> Result<(), LaunchError> {
        self.identity.verify_started(child)
    }
    /// Test-only fixed-plan launch; the diagnostic retains this Plan and actual
    /// Child. This exposes neither a generic command nor a ready-engine token.
    pub(crate) fn spawn_for_diagnostic(
        &self,
    ) -> Result<vessel_proc::Spawned, vessel_proc::SpawnError> {
        vessel_proc::spawn(&self.command)
    }
    /// The supervisor must retain both values; caller cancellation cannot release the profile.
    pub(crate) fn into_supervised_parts(self) -> (CommandSpec, Profile) {
        (self.command, self.profile)
    }
}
