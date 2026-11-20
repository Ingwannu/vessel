//! The installations file's shape (`plugins/installations.json`, ADR 0115):
//! Paseo's `plugins: Record<PluginId, {source, path, enabled?}>`
//! (`packages/protocol/src/plugin-config.ts`), kept by the plugin host alone.
//!
//! The store owns where and how it is written (owner-only, atomic); this
//! module owns what is in it, and a version number so a later build's file
//! is recognised rather than misread.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use vessel_proto::plugins::PluginId;
use vessel_store::DataDir;

use super::source::Directory;

/// The file's format.
const FORMAT: u32 = 1;

/// One installation as kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Installation {
    /// Where its files are.
    pub(crate) source: SavedSource,
    /// The person's intent; Paseo's default for a missing value is true.
    #[serde(default = "enabled_by_default")]
    pub(crate) enabled: bool,
}

const fn enabled_by_default() -> bool {
    true
}

/// A kept source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum SavedSource {
    /// A directory on this node; never deleted by remove.
    Directory {
        /// Absolute, as the person named it (`~` expanded).
        path: PathBuf,
    },
}

impl SavedSource {
    /// The directory the plugin runs from, if the kept path is usable.
    pub(crate) fn directory(&self) -> Option<Directory> {
        match self {
            Self::Directory { path } => Directory::stored(path.clone()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct File {
    format: u32,
    #[serde(default)]
    installations: BTreeMap<PluginId, Installation>,
}

/// Why the file could not be used.
#[derive(Debug, thiserror::Error)]
pub(crate) enum StoreFileError {
    /// Reading or writing failed.
    #[error(transparent)]
    Store(#[from] vessel_store::StoreError),
    /// Not this format.
    #[error("plugins/installations.json is not a file this build reads: {0}")]
    Format(String),
}

/// Read the installations; none when there is no file. Blocking.
pub(crate) fn read(data: &DataDir) -> Result<BTreeMap<PluginId, Installation>, StoreFileError> {
    let Some(bytes) = data.read_plugin_installations()? else {
        return Ok(BTreeMap::new());
    };
    let file: File = serde_json::from_slice(&bytes)
        .map_err(|error| StoreFileError::Format(error.to_string()))?;
    if file.format != FORMAT {
        return Err(StoreFileError::Format(format!(
            "format {} (this build reads {FORMAT})",
            file.format
        )));
    }
    Ok(file.installations)
}

/// Replace the installations. Blocking.
pub(crate) fn write(
    data: &DataDir,
    installations: &BTreeMap<PluginId, Installation>,
) -> Result<(), StoreFileError> {
    let file = File {
        format: FORMAT,
        installations: installations.clone(),
    };
    let mut bytes = serde_json::to_vec_pretty(&file)
        .map_err(|error| StoreFileError::Format(error.to_string()))?;
    bytes.push(b'\n');
    data.write_plugin_installations(&bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installations_round_trip_and_a_later_format_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let data = DataDir::new(root.path());
        assert!(read(&data).unwrap().is_empty());
        let mut installations = BTreeMap::new();
        let _none = installations.insert(
            "theme".parse::<PluginId>().unwrap(),
            Installation {
                source: SavedSource::Directory {
                    path: "/plugins/theme".into(),
                },
                enabled: false,
            },
        );
        write(&data, &installations).unwrap();
        assert_eq!(read(&data).unwrap(), installations);
        data.write_plugin_installations(br#"{"format":2,"installations":{}}"#)
            .unwrap();
        assert!(matches!(read(&data), Err(StoreFileError::Format(_))));
        data.write_plugin_installations(
            br#"{"format":1,"installations":{"x":{"source":{"kind":"directory","path":"/p"}}}}"#,
        )
        .unwrap();
        let read_back = read(&data).unwrap();
        assert!(read_back.values().all(|installation| installation.enabled));
    }
}
