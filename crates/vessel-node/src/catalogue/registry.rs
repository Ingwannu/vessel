//! Reading the ACP registry's JSON (`agent.catalogue`, ADR 0032): the
//! envelope strictly, each entry on its own, so one entry Vessel cannot
//! read is skipped and counted rather than losing the catalogue.
//!
//! [Decision Log]
//! - Purpose: the catalogue shows exactly what adding an entry would run,
//!   and nothing that the registry could make run otherwise later.
//! - Shape (the registry's v1 format, as GOAT's decoder reads it): `{
//!   version, agents: [{ id, name, version, description, license?,
//!   website?, repository?, distribution: { npx?: { package, args?, env? },
//!   uvx?: { package, args?, env? }, binary?: { "<os>-<arch>": { archive,
//!   cmd, sha256?, args?, env? } } } }] }`.
//! - Pins: a package runner fetches what its package names, so a package
//!   without an exact version (`@latest`, a range) would run whatever is
//!   published next; such an entry is skipped. `npx` comes first (it is
//!   what most entries give), then `uvx`, then an archive for this platform
//!   (shown, never downloaded).
//! - Bounds: the contract's ("Bounded, and by how much"): 1,000 entries,
//!   then each string's own; https URLs without credentials only.
//! - One of the four (its vendor's npm package, as `vessel init` installs
//!   it) is shown as that agent, to install and use as itself.

use serde_json::{Map, Value};
use vessel_proto::agents::{
    CatalogueEntry, CatalogueRun, CatalogueVariable, RegistryId, RegistryVersion, Sha256Hex,
};
use vessel_proto::{HarnessKind, HttpUrl};

/// The most entries read.
pub(crate) const MAX_ENTRIES: usize = 1_000;
const MAX_NAME: usize = 160;
const MAX_DESCRIPTION: usize = 1_024;
const MAX_LICENSE: usize = 128;
const MAX_PACKAGE: usize = 256;
const MAX_ARGS: usize = 64;
const MAX_ARG: usize = 1_024;
const MAX_VARIABLES: usize = 64;
const MAX_URL: usize = 2_048;

/// The registry as read: its version, the entries Vessel reads, and how
/// many it skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Read {
    pub(crate) version: RegistryVersion,
    pub(crate) entries: Vec<CatalogueEntry>,
    pub(crate) skipped: u32,
}

/// The registry's envelope is not its shape.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("the registry's answer is not the registry's shape ({0})")]
pub(crate) struct NotTheRegistry(pub(crate) &'static str);

/// Where this node runs, as the registry names platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Platform(pub(crate) &'static str);

impl Platform {
    /// This build's platform, `None` where the registry names none.
    pub(crate) fn here() -> Option<Self> {
        let os = match std::env::consts::OS {
            "linux" => "linux",
            "macos" => "darwin",
            "windows" => "windows",
            _ => return None,
        };
        let target = match (os, std::env::consts::ARCH) {
            ("linux", "x86_64") => "linux-x86_64",
            ("linux", "aarch64") => "linux-aarch64",
            ("darwin", "x86_64") => "darwin-x86_64",
            ("darwin", "aarch64") => "darwin-aarch64",
            ("windows", "x86_64") => "windows-x86_64",
            ("windows", "aarch64") => "windows-aarch64",
            _ => return None,
        };
        Some(Self(target))
    }
}

/// Read `bytes`, for `platform`, telling the four by their packages
/// (`known`) and whether a runner is found (`found`).
///
/// # Errors
/// [`NotTheRegistry`] when the envelope is not JSON of the registry's shape.
pub(crate) fn read(
    bytes: &[u8],
    platform: Option<Platform>,
    known: &[(&str, HarnessKind)],
    found: impl Fn(&str) -> bool,
) -> Result<Read, NotTheRegistry> {
    let Ok(Value::Object(envelope)) = serde_json::from_slice::<Value>(bytes) else {
        return Err(NotTheRegistry("not a JSON object"));
    };
    let version = text(&envelope, "version")
        .and_then(|version| version.parse::<RegistryVersion>().ok())
        .ok_or(NotTheRegistry("no version"))?;
    let Some(Value::Array(agents)) = envelope.get("agents") else {
        return Err(NotTheRegistry("no agents"));
    };
    let mut entries = Vec::new();
    let mut skipped: u32 = 0;
    for (at, agent) in agents.iter().enumerate() {
        let entry = (at < MAX_ENTRIES)
            .then(|| entry(agent, platform, known, &found))
            .flatten();
        match entry {
            Some(entry) => entries.push(entry),
            None => skipped = skipped.saturating_add(1),
        }
    }
    Ok(Read {
        version,
        entries,
        skipped,
    })
}

/// One entry, or `None` when Vessel does not read it.
fn entry(
    agent: &Value,
    platform: Option<Platform>,
    known: &[(&str, HarnessKind)],
    found: &impl Fn(&str) -> bool,
) -> Option<CatalogueEntry> {
    let Value::Object(agent) = agent else {
        return None;
    };
    let id: RegistryId = text(agent, "id")?.parse().ok()?;
    let name = bounded(text(agent, "name")?, MAX_NAME)?.trim().to_owned();
    if name.is_empty() {
        return None;
    }
    let version: RegistryVersion = text(agent, "version")?.parse().ok()?;
    let description = bounded(text(agent, "description")?, MAX_DESCRIPTION)?.to_owned();
    let license = optional(agent, "license", |license| {
        bounded(license, MAX_LICENSE).map(str::to_owned)
    })
    .ok()?;
    let website = optional(agent, "website", https).ok()?;
    let repository = optional(agent, "repository", https).ok()?;
    let Some(Value::Object(distribution)) = agent.get("distribution") else {
        return None;
    };
    let runs = runs(distribution, platform, known, found)?;
    Some(CatalogueEntry {
        id,
        name,
        version,
        description,
        website,
        repository,
        license,
        runs,
    })
}

/// What the entry would run here; `None` when a distribution it gives is
/// not readable (an unpinned package among them).
fn runs(
    distribution: &Map<String, Value>,
    platform: Option<Platform>,
    known: &[(&str, HarnessKind)],
    found: &impl Fn(&str) -> bool,
) -> Option<CatalogueRun> {
    let runners: [(&str, Pinned); 2] = [("npx", npx_pinned), ("uvx", uvx_pinned)];
    for (runner, pinned) in runners {
        let Some(given) = distribution.get(runner) else {
            continue;
        };
        let Value::Object(given) = given else {
            return None;
        };
        let package = bounded(text(given, "package")?, MAX_PACKAGE)?;
        if !pinned(package) {
            return None;
        }
        if runner == "npx"
            && let Some(harness) = known_package(package, known)
        {
            return Some(CatalogueRun::Known { harness });
        }
        let mut args = match runner {
            "npx" => vec!["-y".to_owned(), package.to_owned()],
            _ => vec![package.to_owned()],
        };
        args.extend(arguments(given)?);
        if args.len() > MAX_ARGS {
            return None;
        }
        return Some(CatalogueRun::Command {
            program: runner.parse().ok()?,
            args,
            env: variables(given)?,
            runner_found: found(runner),
        });
    }
    if let Some(binary) = distribution.get("binary") {
        let Value::Object(targets) = binary else {
            return None;
        };
        let target = platform.and_then(|platform| targets.get(platform.0));
        if let Some(target) = target {
            let Value::Object(target) = target else {
                return None;
            };
            let archive = https(text(target, "archive")?)?;
            let sha256 = optional(target, "sha256", |hash| {
                hash.to_ascii_lowercase().parse::<Sha256Hex>().ok()
            })
            .ok()?;
            return Some(CatalogueRun::Download { archive, sha256 });
        }
    }
    Some(CatalogueRun::Unavailable)
}

/// The arguments a distribution adds, within bounds.
fn arguments(given: &Map<String, Value>) -> Option<Vec<String>> {
    match given.get("args") {
        None => Some(Vec::new()),
        Some(Value::Array(args)) if args.len() <= MAX_ARGS => args
            .iter()
            .map(|arg| {
                arg.as_str()
                    .filter(|arg| arg.len() <= MAX_ARG)
                    .map(str::to_owned)
            })
            .collect(),
        Some(_) => None,
    }
}

/// The variables a distribution sets, within bounds.
fn variables(given: &Map<String, Value>) -> Option<Vec<CatalogueVariable>> {
    match given.get("env") {
        None => Some(Vec::new()),
        Some(Value::Object(env)) if env.len() <= MAX_VARIABLES => env
            .iter()
            .map(|(name, value)| {
                let value = value.as_str().filter(|value| value.len() <= MAX_ARG)?;
                Some(CatalogueVariable {
                    name: name.parse().ok()?,
                    value: value.to_owned(),
                })
            })
            .collect(),
        Some(_) => None,
    }
}

/// Whether a runner's package names an exact version.
type Pinned = fn(&str) -> bool;

/// `@scope/name@1.2.3` or `name@1.2.3`: an exact version.
fn npx_pinned(package: &str) -> bool {
    let Some((name, version)) = package.rsplit_once('@') else {
        return false;
    };
    let name_ok = match name.strip_prefix('@') {
        Some(scoped) => scoped
            .split_once('/')
            .is_some_and(|(scope, name)| !scope.is_empty() && !name.is_empty()),
        None => !name.is_empty(),
    };
    name_ok && !name.contains(char::is_whitespace) && exact(version)
}

/// `name==1.2.3` or `name@1.2.3`.
fn uvx_pinned(package: &str) -> bool {
    let split = package.split_once("==").or_else(|| package.split_once('@'));
    split.is_some_and(|(name, version)| {
        name.starts_with(|c: char| c.is_ascii_alphanumeric())
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
            && exact(version)
    })
}

/// `1.2.3`, `v1.2.3`, with an optional pre-release and build.
fn exact(version: &str) -> bool {
    let version = version.strip_prefix('v').unwrap_or(version);
    let (core, _build) = version.split_once('+').unwrap_or((version, ""));
    let (core, _pre) = core.split_once('-').unwrap_or((core, ""));
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
}

/// Which of the four `package` (`name@version`) is.
fn known_package(package: &str, known: &[(&str, HarnessKind)]) -> Option<HarnessKind> {
    let (name, _version) = package.rsplit_once('@')?;
    known
        .iter()
        .find(|(known, _harness)| *known == name)
        .map(|(_known, harness)| *harness)
}

fn text<'a>(object: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    object.get(key).and_then(Value::as_str)
}

/// `text` when it has at most `max` characters.
fn bounded(text: &str, max: usize) -> Option<&str> {
    (text.chars().count() <= max).then_some(text)
}

/// A field there but not readable, which skips the entry.
struct Unreadable;

/// `Ok(None)` when absent, `Ok(Some(_))` when read.
fn optional<T>(
    object: &Map<String, Value>,
    key: &str,
    read: impl Fn(&str) -> Option<T>,
) -> Result<Option<T>, Unreadable> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value.as_str().and_then(read).map(Some).ok_or(Unreadable),
    }
}

/// An https URL without credentials, within its bound.
fn https(url: &str) -> Option<HttpUrl> {
    if url.len() > MAX_URL || !url.starts_with("https://") {
        return None;
    }
    url.parse().ok()
}
