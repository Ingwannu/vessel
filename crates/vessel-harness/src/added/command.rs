//! [`command`]: what an added agent runs as, from an empty environment.
//!
//! [Decision Log]
//! - Purpose: an added agent gets what its definition names and the
//!   system's own variables a program needs to run, and none of the
//!   node's credentials (ADR 0032).
//! - Constraint: the node's environment holds provider keys, tokens and
//!   Vessel's own variables; an added agent is often a package fetched from
//!   the network at its first start.
//! - Alternatives: the node's environment minus a list of known credential
//!   names (Paseo removes four; any name not on the list leaks); the
//!   node's environment whole (the four's case, whose vendors the person
//!   chose when installing them).
//! - Choice: start empty; add `PATH` (the search path the program was found
//!   on), the user's home from `HarnessEnv`, the variables of
//!   [`PASSED_ON`] the node has (proxies only without a user and password
//!   in them), the definition's `inherit`, its `env` (secrets filled in by
//!   the caller), and last the profile's own changes.
//! - Left out on purpose, though the four inherit them: the keyring's bus
//!   (`DBUS_SESSION_BUS_ADDRESS`) and the SSH agent's socket
//!   (`SSH_AUTH_SOCK`), each a way to other credentials. `inherit` passes
//!   them when the owner wants.
//! - What a test can check: [`command`] takes the node's environment as a
//!   reader, as `home` does, so a test gives it one of its own.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};

use vessel_proc::CommandSpec;
use vessel_proto::{ConfigValue, ResolvedValue, SecretName};

use super::AddedAgent;
use crate::env::HarnessEnv;
use crate::error::HarnessError;
use crate::lookup::Lookup;

/// The system's own variables an added agent is given when the node has
/// them: none holds a credential. Proxies are checked one by one
/// ([`passed_on`]).
#[cfg(unix)]
pub const PASSED_ON: &[&str] = &[
    "USER",
    "LOGNAME",
    "SHELL",
    "LANG",
    "LANGUAGE",
    "LC_ALL",
    "LC_CTYPE",
    "LC_MESSAGES",
    "TERM",
    "TZ",
    "TMPDIR",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_STATE_HOME",
    "XDG_CACHE_HOME",
    "XDG_RUNTIME_DIR",
    "__CF_USER_TEXT_ENCODING",
    "SSL_CERT_FILE",
    "SSL_CERT_DIR",
    "NODE_EXTRA_CA_CERTS",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "NO_PROXY",
    "ALL_PROXY",
    "http_proxy",
    "https_proxy",
    "no_proxy",
    "all_proxy",
];

/// The system's own variables an added agent is given when the node has
/// them: none holds a credential. Proxies are checked one by one
/// ([`passed_on`]). Windows compares names ignoring case, so each proxy is
/// listed once.
#[cfg(windows)]
pub const PASSED_ON: &[&str] = &[
    "USERNAME",
    "USERDOMAIN",
    "COMPUTERNAME",
    "HOMEDRIVE",
    "HOMEPATH",
    "SystemRoot",
    "SystemDrive",
    "windir",
    "ComSpec",
    "PATHEXT",
    "TEMP",
    "TMP",
    "APPDATA",
    "LOCALAPPDATA",
    "ProgramData",
    "ALLUSERSPROFILE",
    "ProgramFiles",
    "ProgramFiles(x86)",
    "ProgramW6432",
    "CommonProgramFiles",
    "CommonProgramFiles(x86)",
    "CommonProgramW6432",
    "PSModulePath",
    "NUMBER_OF_PROCESSORS",
    "PROCESSOR_ARCHITECTURE",
    "PROCESSOR_IDENTIFIER",
    "PROCESSOR_LEVEL",
    "PROCESSOR_REVISION",
    "OS",
    "SSL_CERT_FILE",
    "SSL_CERT_DIR",
    "NODE_EXTRA_CA_CERTS",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "NO_PROXY",
    "ALL_PROXY",
];

/// Whether the node's `value` of `name` (one of [`PASSED_ON`]) is passed
/// on: a proxy whose value holds a user and password (`user:pass@host`)
/// is a credential, and is not.
#[must_use]
pub fn passed_on(name: &str, value: &OsStr) -> bool {
    let proxy = name.to_ascii_uppercase().ends_with("_PROXY");
    !proxy || !value.as_encoded_bytes().contains(&b'@')
}

/// The command `agent` runs as, before its profile's changes: its program
/// (looked up on `lookup`, or the absolute path written) and arguments, in
/// an environment that starts empty (the module's Decision Log). `node`
/// reads the node's own environment (the binaries pass the process's);
/// `secrets` holds the value of every secret the agent names.
///
/// Does filesystem lookups; call it from a blocking context.
///
/// # Errors
/// [`HarnessError::AgentMissing`] when the program is not found;
/// [`HarnessError::SecretMissing`] when a secret it names has no value.
pub fn command(
    agent: &AddedAgent,
    lookup: &Lookup,
    env: &HarnessEnv,
    secrets: &BTreeMap<SecretName, ResolvedValue>,
    node: impl Fn(&str) -> Option<OsString>,
) -> Result<CommandSpec, HarnessError> {
    let program =
        crate::discover::found(agent, lookup).ok_or_else(|| HarnessError::AgentMissing {
            name: agent.name().clone(),
            program: agent.program().clone(),
        })?;
    let mut command = CommandSpec::new(program)
        .args(agent.args.iter())
        .env_clear();
    if let Some(path) = lookup.search_path() {
        command = command.env("PATH", path);
    }
    let home = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    command = command.env(home, env.user_home());
    for name in PASSED_ON {
        if let Some(value) = node(name)
            && passed_on(name, &value)
        {
            command = command.env(name, value);
        }
    }
    for name in &agent.inherit {
        if let Some(value) = node(name.as_str()) {
            command = command.env(name.as_str(), value);
        }
    }
    for (name, value) in &agent.env {
        let value = match value {
            ConfigValue::Text(text) => text.as_str(),
            ConfigValue::Secret(secret) => secrets
                .get(secret)
                .ok_or_else(|| HarnessError::SecretMissing {
                    name: agent.name().clone(),
                    secret: secret.clone(),
                })?
                .expose(),
        };
        command = command.env(name.as_str(), value);
    }
    Ok(command)
}

/// The command that runs `install` (one of the four's documented
/// installers, `crate::installs`) on the node, for `agent.install`
/// (protocol 11): its program found on `lookup`, in the user's home, from
/// the environment an added agent starts with (`PATH`, the home,
/// [`PASSED_ON`]) plus the node's `npm_config_*` variables (`node_vars`,
/// the node's whole environment), so a package's install script sees no
/// provider key. `None` when its program (`npm`) is not found.
///
/// Does filesystem lookups; call it from a blocking context.
#[must_use]
pub fn installer(
    install: &crate::setup::Install,
    lookup: &Lookup,
    env: &HarnessEnv,
    node: impl Fn(&str) -> Option<OsString>,
    node_vars: impl IntoIterator<Item = (OsString, OsString)>,
) -> Option<CommandSpec> {
    let program = lookup.find(install.program)?;
    let mut command = CommandSpec::new(program)
        .args(install.args())
        .env_clear()
        .cwd(env.user_home());
    if let Some(path) = lookup.search_path() {
        command = command.env("PATH", path);
    }
    let home = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    command = command.env(home, env.user_home());
    for name in PASSED_ON {
        if let Some(value) = node(name)
            && passed_on(name, &value)
        {
            command = command.env(name, value);
        }
    }
    for (name, value) in node_vars {
        if name
            .to_str()
            .is_some_and(|name| name.to_ascii_lowercase().starts_with("npm_config_"))
        {
            command = command.env(name, value);
        }
    }
    Some(command)
}
