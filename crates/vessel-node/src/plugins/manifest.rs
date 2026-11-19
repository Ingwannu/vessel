//! `vessel-plugin.json`, the manifest of a plugin directory (ADR 0115).
//!
//! [Decision Log]
//! - Purpose: Paseo reads `paseo-plugin.json` (`id`, `name`, `description`,
//!   `icon`, `media`, `requirements`, `build`;
//!   `packages/server/src/server/plugins/manifest.ts:7-46`) and finds its
//!   entries as `index.server.ts` / `index.client.tsx`. Vessel cannot run
//!   those entries (ADR 0079), so its manifest names them: `server.command`
//!   (a helper program's argv) and `client` (a static declarative document).
//! - Same field rules as Paseo where the field is Paseo's: id pattern,
//!   trimmed non-empty name and description, asset paths that stay inside
//!   the directory (no leading `/`, no `..`, no `\ : ? #` or controls), a
//!   PNG icon, `media` as asset paths or https URLs, strict
//!   `requirements` checked before anything runs, `build` as argv lists.
//! - Unknown fields are refused (ADR 0115), where Paseo's schema drops
//!   them: a misspelt `server` must not install a plugin that never starts.
//! - A directory holding only `paseo-plugin.json` is refused with a
//!   sentence that says why, rather than "manifest missing".

use std::path::{Component, Path, PathBuf};

use serde::Deserialize;
use vessel_proto::plugins::{Caption, ContentText, PluginId};

/// The manifest's file name.
pub(crate) const MANIFEST_FILE: &str = "vessel-plugin.json";
/// Paseo's, recognised only to explain the refusal.
const PASEO_MANIFEST_FILE: &str = "paseo-plugin.json";
/// A manifest larger than this is not read.
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;

/// Why a directory is not a plugin this node installs.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ManifestError {
    /// A Paseo plugin (stated difference, ADR 0115).
    #[error(
        "this is a Paseo plugin (paseo-plugin.json). Vessel runs plugins described by vessel-plugin.json: a helper program and declarative client data; Paseo's TypeScript entries do not run here"
    )]
    Paseo,
    /// No manifest.
    #[error("plugin manifest is missing: {0}")]
    Missing(String),
    /// Unreadable, too large, not JSON, or a field out of its rules.
    #[error("plugin manifest is invalid: {0}")]
    Invalid(String),
    /// Neither `server` nor `client`.
    #[error("plugin manifest is invalid: it names neither a server command nor a client document")]
    NoEntry,
    /// `requirements.vessel` is not met by this node.
    #[error("plugin requires Vessel {required}; this node is {version}")]
    Unmet {
        /// The range.
        required: String,
        /// This node's version.
        version: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    id: PluginId,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    media: Option<Vec<String>>,
    #[serde(default)]
    requirements: Option<RawRequirements>,
    #[serde(default)]
    build: Option<Vec<Vec<String>>>,
    #[serde(default)]
    server: Option<RawServer>,
    #[serde(default)]
    client: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRequirements {
    #[serde(default)]
    vessel: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawServer {
    command: Vec<String>,
}

/// A checked manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Manifest {
    /// The default installation id.
    pub(crate) id: PluginId,
    /// Shown in the list.
    pub(crate) name: Option<Caption>,
    /// Shown in the list.
    pub(crate) description: Option<ContentText>,
    /// Relative PNG path, listed as Paseo lists it.
    pub(crate) icon: Option<String>,
    /// The helper's argv, as written (resolved when it starts).
    pub(crate) server: Option<Vec<String>>,
    /// The client document's path inside the directory.
    pub(crate) client: Option<PathBuf>,
}

/// Read and check `directory`'s manifest against this node's `version`.
/// Blocking.
pub(crate) fn read(directory: &Path, version: &semver::Version) -> Result<Manifest, ManifestError> {
    let path = directory.join(MANIFEST_FILE);
    let metadata = match std::fs::metadata(&path) {
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) | Err(_) => {
            return Err(if directory.join(PASEO_MANIFEST_FILE).is_file() {
                ManifestError::Paseo
            } else {
                ManifestError::Missing(path.display().to_string())
            });
        }
    };
    if metadata.len() > MAX_MANIFEST_BYTES {
        return Err(ManifestError::Invalid(format!(
            "{MANIFEST_FILE} is larger than {MAX_MANIFEST_BYTES} bytes"
        )));
    }
    let bytes = std::fs::read(&path)
        .map_err(|error| ManifestError::Invalid(format!("{MANIFEST_FILE}: {error}")))?;
    parse(&bytes, directory, version)
}

/// Check manifest `bytes` of `directory`.
pub(crate) fn parse(
    bytes: &[u8],
    directory: &Path,
    version: &semver::Version,
) -> Result<Manifest, ManifestError> {
    let raw: Raw =
        serde_json::from_slice(bytes).map_err(|error| ManifestError::Invalid(error.to_string()))?;
    let name = trimmed(raw.name, "name")?
        .map(|name| {
            name.parse::<Caption>()
                .map_err(|error| ManifestError::Invalid(format!("name: {error}")))
        })
        .transpose()?;
    let description = trimmed(raw.description, "description")?
        .map(|text| {
            text.parse::<ContentText>()
                .map_err(|error| ManifestError::Invalid(format!("description: {error}")))
        })
        .transpose()?;
    if let Some(icon) = &raw.icon {
        asset(icon, "icon")?;
        let png = Path::new(icon)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("png"));
        if !png {
            return Err(ManifestError::Invalid(
                "icon: expected a relative path to a PNG".to_owned(),
            ));
        }
    }
    for item in raw.media.iter().flatten() {
        if !item.starts_with("https://") {
            asset(item, "media")?;
        }
    }
    for command in raw.build.iter().flatten() {
        if command.is_empty() || command.iter().any(|argument| argument.trim().is_empty()) {
            return Err(ManifestError::Invalid(
                "build: every command is a non-empty list of non-empty arguments".to_owned(),
            ));
        }
    }
    if let Some(range) = raw
        .requirements
        .and_then(|requirements| requirements.vessel)
    {
        let required = semver::VersionReq::parse(&range)
            .map_err(|error| ManifestError::Invalid(format!("requirements.vessel: {error}")))?;
        if !required.matches(version) {
            return Err(ManifestError::Unmet {
                required: range,
                version: version.to_string(),
            });
        }
    }
    let server = match raw.server {
        Some(server) if server.command.is_empty() => {
            return Err(ManifestError::Invalid(
                "server.command: the program is missing".to_owned(),
            ));
        }
        Some(server) => Some(server.command),
        None => None,
    };
    let client = raw
        .client
        .map(|client| asset(&client, "client").map(|()| directory.join(client)))
        .transpose()?;
    if server.is_none() && client.is_none() {
        return Err(ManifestError::NoEntry);
    }
    Ok(Manifest {
        id: raw.id,
        name,
        description,
        icon: raw.icon,
        server,
        client,
    })
}

/// A present field trimmed, refused when nothing is left (Paseo's
/// `z.string().trim().min(1)`).
fn trimmed(text: Option<String>, field: &str) -> Result<Option<String>, ManifestError> {
    match text {
        None => Ok(None),
        Some(text) if text.trim().is_empty() => Err(ManifestError::Invalid(format!(
            "{field}: expected text, not an empty string"
        ))),
        Some(text) => Ok(Some(text.trim().to_owned())),
    }
}

/// Paseo's `PluginAssetPathSchema`: a relative file path that stays inside
/// the plugin directory.
fn asset(path: &str, field: &str) -> Result<(), ManifestError> {
    let refused = path.is_empty()
        || path != path.trim()
        || path.starts_with('/')
        || path.ends_with('/')
        || path
            .chars()
            .any(|c| c.is_control() || matches!(c, '\\' | ':' | '?' | '#'))
        || path.split('/').any(|segment| segment == "..")
        || path.rsplit('/').next() == Some(".")
        || Path::new(path)
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir));
    if refused {
        return Err(ManifestError::Invalid(format!(
            "{field}: expected a relative file path inside the plugin directory"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version() -> semver::Version {
        semver::Version::new(0, 12, 0)
    }

    fn checked(json: &str) -> Result<Manifest, ManifestError> {
        parse(json.as_bytes(), Path::new("/plugins/theme"), &version())
    }

    #[test]
    fn a_client_only_manifest_with_paseo_fields() {
        let manifest = checked(
            r#"{"id":"theme","name":"  Night  ","description":"A theme","icon":"assets/icon.PNG",
                "media":["shot.png","https://example.com/a.png"],"client":"client.json",
                "requirements":{"vessel":">=0.11"}}"#,
        )
        .unwrap();
        assert_eq!(manifest.id.as_str(), "theme");
        assert_eq!(manifest.name.unwrap().as_str(), "Night");
        assert_eq!(
            manifest.client,
            Some(PathBuf::from("/plugins/theme/client.json"))
        );
        assert_eq!(manifest.server, None);
    }

    #[test]
    fn entries_requirements_and_unknown_fields() {
        assert_eq!(checked(r#"{"id":"x"}"#), Err(ManifestError::NoEntry));
        assert!(matches!(
            checked(r#"{"id":"x","client":"c.json","sever":{"command":["a"]}}"#),
            Err(ManifestError::Invalid(_))
        ));
        assert!(matches!(
            checked(r#"{"id":"x","client":"c.json","requirements":{"paseo":">=1"}}"#),
            Err(ManifestError::Invalid(_))
        ));
        assert_eq!(
            checked(r#"{"id":"x","client":"c.json","requirements":{"vessel":">=1"}}"#),
            Err(ManifestError::Unmet {
                required: ">=1".to_owned(),
                version: "0.12.0".to_owned()
            })
        );
        assert!(matches!(
            checked(r#"{"id":"x","server":{"command":[]}}"#),
            Err(ManifestError::Invalid(_))
        ));
        assert!(matches!(
            checked(r#"{"id":"Bad","client":"c.json"}"#),
            Err(ManifestError::Invalid(_))
        ));
    }

    #[test]
    fn asset_paths_stay_inside_the_directory() {
        for path in [
            "../c.json",
            "/etc/c.json",
            "a/../c.json",
            "c\\d.json",
            "a:b",
            " c.json",
            "dir/",
            ".",
        ] {
            assert!(
                matches!(
                    checked(&format!(r#"{{"id":"x","client":{path:?}}}"#)),
                    Err(ManifestError::Invalid(_))
                ),
                "{path}"
            );
        }
        assert!(matches!(
            checked(r#"{"id":"x","client":"c.json","icon":"icon.svg"}"#),
            Err(ManifestError::Invalid(_))
        ));
        assert!(matches!(
            checked(r#"{"id":"x","client":"c.json","name":"   "}"#),
            Err(ManifestError::Invalid(_))
        ));
    }

    #[test]
    fn a_paseo_plugin_is_refused_with_a_reason() {
        let directory = tempfile::tempdir().unwrap();
        assert!(matches!(
            read(directory.path(), &version()),
            Err(ManifestError::Missing(_))
        ));
        std::fs::write(directory.path().join(PASEO_MANIFEST_FILE), br#"{"id":"x"}"#).unwrap();
        let refused = read(directory.path(), &version()).unwrap_err();
        assert_eq!(refused, ManifestError::Paseo);
        assert!(refused.to_string().contains("vessel-plugin.json"));
        std::fs::write(
            directory.path().join(MANIFEST_FILE),
            br#"{"id":"x","client":"c.json"}"#,
        )
        .unwrap();
        assert_eq!(read(directory.path(), &version()).unwrap().id.as_str(), "x");
    }
}
