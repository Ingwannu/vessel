//! `vessel.toml`'s `[scripts.<name>]` and `[services.<name>]` (protocol
//! 11, ADR 0031): a project's named commands, and how their approval is
//! digested.
//!
//! [Decision Log]
//! - Shape: Paseo's `scripts.<name> { command, type }` as two TOML tables,
//!   so the kind is where a person reads it; an optional `dir` inside the
//!   root. An unknown key inside an entry is ignored, as ADR 0027 ignores
//!   every key it does not read: a later node's key must not stop this
//!   one's setup. A missing `command`, a name that is not a `ScriptName`,
//!   a name in both tables, or more than [`MAX_SCRIPTS`] make the file
//!   invalid, and then nothing in it runs.
//! - The digest grows only where the file does: one without scripts keeps
//!   ADR 0027's version 1 digest exactly, so an approval given before
//!   scripts existed still covers what it showed; one with scripts is
//!   version 2, which no earlier approval can match.

use std::collections::BTreeMap;

use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use vessel_proto::scripts::{InvalidScriptEntry, ScriptEntry, ScriptKind, ScriptName};
use vessel_proto::workspace::{
    InvalidRelPath, InvalidServicePort, InvalidText, ProjectCommand, RelPath, ServicePort,
};

/// The most scripts and services one file names, together (GOAT allows 50).
pub const MAX_SCRIPTS: usize = 32;

/// One entry as written.
#[derive(Debug, Deserialize)]
pub(super) struct RawScript {
    command: Option<String>,
    dir: Option<String>,
    port: Option<i64>,
}

/// The two tables as written.
pub(super) type RawTable = BTreeMap<String, RawScript>;

/// What is wrong with a file's scripts. Nothing in the file runs then.
#[derive(Debug, thiserror::Error)]
pub enum ScriptsError {
    /// A declaration cannot be represented as a valid script or service.
    #[error("`vessel.toml` script declaration: {0}")]
    Declaration(#[source] InvalidScriptEntry),
    /// More than [`MAX_SCRIPTS`] together.
    #[error("`vessel.toml` names more than {MAX_SCRIPTS} scripts and services")]
    TooMany,
    /// A declaration that cannot identify an executable TCP service port.
    #[error("`{table}.{name}.port`: {source}")]
    Port {
        /// Identify the table whose entry is refused.
        table: &'static str,
        /// Identify the declaration without losing its numeric cause.
        name: ScriptName,
        /// Keep the validating port type's precise refusal.
        source: InvalidServicePort,
    },
    /// A name that is not 1 to 64 letters, digits, `-`, `_` or `.`.
    #[error("`vessel.toml`: {0}")]
    Name(#[source] vessel_proto::scripts::InvalidScriptName),
    /// The same name in `[scripts]` and `[services]`.
    #[error("`vessel.toml` names `{0}` as both a script and a service")]
    Both(ScriptName),
    /// An entry without a `command`.
    #[error("`{table}.{name}` has no `command`")]
    NoCommand {
        /// `scripts` or `services`.
        table: &'static str,
        /// Its name.
        name: ScriptName,
    },
    /// A command the rules refuse (empty, too long, NUL).
    #[error("`{table}.{name}.command`: {source}")]
    Command {
        /// `scripts` or `services`.
        table: &'static str,
        /// Its name.
        name: ScriptName,
        /// What is wrong with it.
        source: InvalidText,
    },
    /// A folder that is not a path inside the root.
    #[error("`{table}.{name}.dir`: {source}")]
    Dir {
        /// `scripts` or `services`.
        table: &'static str,
        /// Its name.
        name: ScriptName,
        /// What is wrong with it.
        source: InvalidRelPath,
    },
}

/// Both tables' entries, in name order, checked.
pub(super) fn entries(
    scripts: RawTable,
    services: RawTable,
) -> Result<Vec<ScriptEntry>, ScriptsError> {
    if scripts.len().saturating_add(services.len()) > MAX_SCRIPTS {
        return Err(ScriptsError::TooMany);
    }
    let mut entries = Vec::with_capacity(scripts.len().saturating_add(services.len()));
    for (kind, table) in [
        (ScriptKind::Script, scripts),
        (ScriptKind::Service, services),
    ] {
        for (name, raw) in table {
            entries.push(entry(kind, &name, raw)?);
        }
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    if let Some(pair) = entries.windows(2).find(|pair| match pair {
        [first, second] => first.name == second.name,
        _ => false,
    }) && let Some(first) = pair.first()
    {
        return Err(ScriptsError::Both(first.name.clone()));
    }
    Ok(entries)
}

fn entry(kind: ScriptKind, name: &str, raw: RawScript) -> Result<ScriptEntry, ScriptsError> {
    let table = kind.table();
    let name: ScriptName = name.parse().map_err(ScriptsError::Name)?;
    let command = raw.command.ok_or_else(|| ScriptsError::NoCommand {
        table,
        name: name.clone(),
    })?;
    let command: ProjectCommand = command.parse().map_err(|source| ScriptsError::Command {
        table,
        name: name.clone(),
        source,
    })?;
    let dir = raw
        .dir
        .map(|dir| dir.parse::<RelPath>())
        .transpose()
        .map_err(|source| ScriptsError::Dir {
            table,
            name: name.clone(),
            source,
        })?;
    let port = raw
        .port
        .map(ServicePort::try_from)
        .transpose()
        .map_err(|source| ScriptsError::Port {
            table,
            name: name.clone(),
            source,
        })?;
    ScriptEntry::new(name, kind, command, dir, port).map_err(ScriptsError::Declaration)
}

/// Add `bytes` to `hash` with its length first (8 bytes little-endian), so
/// no two different sequences of fields hash alike.
pub(super) fn field(hash: &mut Sha256, bytes: &[u8]) {
    hash.update(u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_le_bytes());
    hash.update(bytes);
}

/// Version three reserves a per-script port field; earlier approvals omit it.
#[derive(Clone, Copy)]
pub(super) enum DigestFields {
    /// Preserve the byte encoding of approvals that predate terminal keys.
    Original,
    /// Include the port slot even before port allocation is implemented.
    WithPorts,
}

/// Keep script ordering identical across digest versions.
pub(super) fn digest(hash: &mut Sha256, scripts: &[ScriptEntry], fields: DigestFields) {
    hash.update(
        u64::try_from(scripts.len())
            .unwrap_or(u64::MAX)
            .to_le_bytes(),
    );
    for script in scripts {
        field(hash, script.name.as_str().as_bytes());
        field(hash, script.kind().table().as_bytes());
        field(hash, script.command.as_str().as_bytes());
        field(
            hash,
            script.dir.as_ref().map_or("", RelPath::as_str).as_bytes(),
        );
        match fields {
            DigestFields::Original => {}
            DigestFields::WithPorts => {
                let port = script.port().map(|port| port.to_string());
                field(hash, port.as_deref().unwrap_or("").as_bytes());
            }
        }
    }
}
