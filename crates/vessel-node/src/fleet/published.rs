//! `fleet.published` (protocol 16, ADR 0056): what this machine shares, or
//! last received, through its fleet, by name and metadata, read from its
//! own files on a blocking task. Never file contents, never a secret's
//! value, never a tool server's command, arguments, address or headers.
//!
//! [Decision Log]
//! - Purpose: a person's page shows what a home shares and what a member
//!   last received, without the member protocol (`fleet.manifest`, `sync`).
//! - Source: `manifest.json`, which a home writes when it publishes and a
//!   member when it pulls, and `state.json` for the version a member last
//!   applied; the same files `vessel fleet status` reads, so the answer
//!   holds whether or not the fleet loop is running.
//! - Bounds as `fleet.status` (`projection`): the file is read up to a
//!   byte bound, the bundle up to an item bound, the reply up to the
//!   reply's byte budget; past any of them the whole read is refused
//!   (`TooLarge`), never cut to a shorter list a person would trust.

use std::collections::BTreeSet;

use vessel_proto::{
    BundlePath, ConfigValue, FleetPublished, FleetRole, Manifest, McpTransport, PublishedBundle,
    SharedFile, SharedFileKind,
};

use super::files::FleetFiles;
use super::projection::{self, ProjectionError};
use super::state::FleetState;

/// The largest `manifest.json` read: a bundle of many skills' files, its
/// paths at most 1024 bytes each.
const MANIFEST_BYTES: u64 = 16 << 20;
/// The most files a published list carries.
pub(super) const MAX_FILES: usize = 4096;
/// The most tool servers a published list carries.
pub(super) const MAX_SERVERS: usize = 256;

/// What this machine shares or last received (blocking).
///
/// # Errors
/// A file cannot be read or decoded, or the answer is over a bound.
pub(crate) fn read(files: &FleetFiles) -> Result<FleetPublished, ProjectionError> {
    let published = match projection::role(files)? {
        FleetRole::Standalone => FleetPublished::Standalone,
        FleetRole::Home => FleetPublished::Home {
            bundle: bundle(files)?,
        },
        FleetRole::Member { home, .. } => {
            let state =
                projection::read_json::<FleetState>(&files.state(), projection::STATE_BYTES)?
                    .unwrap_or_default();
            FleetPublished::Member {
                home,
                bundle: bundle(files)?,
                applied: state.generation,
            }
        }
        FleetRole::Unknown => FleetPublished::Unknown,
    };
    projection::within_reply(&published)?;
    Ok(published)
}

/// The bundle in `manifest.json` by name; `None` before there is one.
fn bundle(files: &FleetFiles) -> Result<Option<PublishedBundle>, ProjectionError> {
    let Some(manifest) = projection::read_json::<Manifest>(&files.manifest(), MANIFEST_BYTES)?
    else {
        return Ok(None);
    };
    if manifest.files.len() > MAX_FILES || manifest.mcp.len() > MAX_SERVERS {
        return Err(ProjectionError::TooLarge);
    }
    let secrets: BTreeSet<_> = manifest
        .mcp
        .values()
        .flat_map(McpTransport::values)
        .filter_map(|value| match value {
            ConfigValue::Secret(name) => Some(name.clone()),
            ConfigValue::Text(_) => None,
        })
        .collect();
    Ok(Some(PublishedBundle {
        generation: manifest.generation,
        files: manifest
            .files
            .into_iter()
            .map(|(path, file)| SharedFile {
                kind: kind(&path),
                path,
                size: file.size,
                executable: file.executable,
            })
            .collect(),
        servers: manifest.mcp.into_keys().collect(),
        secrets: secrets.into_iter().collect(),
    }))
}

/// What a file is, from where the bundle puts it (`vessel_fleet`'s layout).
fn kind(path: &BundlePath) -> SharedFileKind {
    let path = path.as_str();
    if path == vessel_fleet::INSTRUCTIONS {
        SharedFileKind::Instructions
    } else if path
        .strip_prefix(vessel_fleet::SKILLS)
        .is_some_and(|rest| rest.starts_with('/'))
    {
        SharedFileKind::Skill
    } else {
        SharedFileKind::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fleet::files;
    use crate::fleet::member::Membership;
    use std::collections::BTreeMap;
    use vessel_proto::{BlobHash, BundleFile, Generation, NodeId, NodePublicKey};

    fn files() -> (tempfile::TempDir, FleetFiles) {
        let temp = tempfile::tempdir().unwrap();
        let files = FleetFiles::new(&vessel_store::DataDir::new(temp.path().join("data")));
        files::create_private_dir(files.dir()).unwrap();
        (temp, files)
    }

    fn manifest(generation: u64) -> Manifest {
        let file = |size, executable| BundleFile {
            hash: BlobHash::from_bytes([1; 32]),
            size,
            executable,
        };
        let mut paths = BTreeMap::new();
        let _none = paths.insert("instructions.md".parse().unwrap(), file(10, false));
        let _none = paths.insert("skills/build/run.sh".parse().unwrap(), file(20, true));
        let _none = paths.insert("skillset.md".parse().unwrap(), file(30, false));
        let mcp = serde_json::from_value(serde_json::json!({
            "github": {
                "type": "stdio",
                "command": "github-mcp",
                "args": ["--verbose"],
                "env": {"MCP_AUTH": {"secret": "GITHUB_TOKEN"}, "MCP_MODE": "plain text"}
            }
        }))
        .unwrap();
        Manifest {
            generation: Generation::new(generation),
            files: paths,
            mcp,
        }
    }

    fn become_home(files: &FleetFiles) {
        std::fs::write(files.sources(), "").unwrap();
    }

    fn become_member(files: &FleetFiles, home: NodeId) {
        files::write_json(
            &files.membership(),
            &Membership {
                home,
                key: NodePublicKey::from_bytes([7; 32]),
                relays: Vec::new(),
                direct: Vec::new(),
                joined_at: chrono::DateTime::UNIX_EPOCH,
                grant: Vec::new(),
            },
        )
        .unwrap();
    }

    #[test]
    fn a_standalone_machine_publishes_nothing_as_a_value() {
        let (_temp, files) = files();
        // A manifest left from an earlier membership is not what it shares.
        files::write_json(&files.manifest(), &manifest(4)).unwrap();
        assert_eq!(read(&files).unwrap(), FleetPublished::Standalone);
    }

    #[test]
    fn a_home_shares_names_kinds_and_sizes_but_no_command_or_value() {
        let (_temp, files) = files();
        become_home(&files);
        assert_eq!(
            read(&files).unwrap(),
            FleetPublished::Home { bundle: None },
            "before its first bundle"
        );
        files::write_json(&files.manifest(), &manifest(2)).unwrap();
        let FleetPublished::Home {
            bundle: Some(bundle),
        } = read(&files).unwrap()
        else {
            panic!("a home's bundle");
        };
        assert_eq!(bundle.generation, Generation::new(2));
        let kinds: Vec<_> = bundle
            .files
            .iter()
            .map(|file| (file.path.as_str(), file.kind, file.size, file.executable))
            .collect();
        assert_eq!(
            kinds,
            [
                ("instructions.md", SharedFileKind::Instructions, 10, false),
                ("skills/build/run.sh", SharedFileKind::Skill, 20, true),
                ("skillset.md", SharedFileKind::Other, 30, false),
            ]
        );
        assert_eq!(bundle.servers, ["github".parse().unwrap()]);
        assert_eq!(bundle.secrets, ["GITHUB_TOKEN".parse().unwrap()]);
        let encoded = serde_json::to_string(&bundle).unwrap();
        for hidden in [
            "github-mcp",
            "--verbose",
            "plain text",
            "MCP_AUTH",
            "MCP_MODE",
        ] {
            assert!(!encoded.contains(hidden), "{hidden} in {encoded}");
        }
    }

    #[test]
    fn a_member_says_where_its_bundle_came_from_and_what_it_applied() {
        let (_temp, files) = files();
        let home = NodeId::random();
        become_member(&files, home);
        assert_eq!(
            read(&files).unwrap(),
            FleetPublished::Member {
                home,
                bundle: None,
                applied: None
            }
        );
        files::write_json(&files.manifest(), &manifest(5)).unwrap();
        files::write_json(
            &files.state(),
            &FleetState {
                generation: Some(Generation::new(4)),
                ..FleetState::default()
            },
        )
        .unwrap();
        let FleetPublished::Member {
            home: from,
            bundle: Some(bundle),
            applied,
        } = read(&files).unwrap()
        else {
            panic!("a member's bundle");
        };
        assert_eq!(from, home);
        assert_eq!(bundle.generation, Generation::new(5));
        assert_eq!(applied, Some(Generation::new(4)), "the last apply lags");
    }

    #[test]
    fn a_bundle_over_a_bound_is_refused_whole() {
        let (_temp, files) = files();
        become_home(&files);
        let mut long = manifest(1);
        let file = long.files.values().next().unwrap().clone();
        for index in 0..=MAX_FILES {
            let _none = long.files.insert(
                format!("skills/s/{index}.md").parse().unwrap(),
                file.clone(),
            );
        }
        files::write_json(&files.manifest(), &long).unwrap();
        assert!(matches!(read(&files), Err(ProjectionError::TooLarge)));
        // Under the item bound, paths long enough to pass the reply's bytes.
        let mut wide = manifest(1);
        let segment = "x".repeat(250);
        for index in 0..MAX_FILES - 10 {
            let path = format!("skills/{segment}/{segment}/{segment}/{index}.md");
            let _none = wide.files.insert(path.parse().unwrap(), file.clone());
        }
        files::write_json(&files.manifest(), &wide).unwrap();
        assert!(matches!(
            read(&files),
            Err(ProjectionError::Reply(_) | ProjectionError::TooLarge)
        ));
    }
}
