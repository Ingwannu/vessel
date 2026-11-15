//! Ownership manifests record only bounded relative paths and content hashes.

use super::filesystem;
use crate::dir::Dir;
use crate::orchestration::bundle::MAX_SKILL_FILES;
use crate::orchestration::{SkillBundle, SkillError};
use serde::de::{Error as _, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::{collections::BTreeMap, fmt};
use vessel_proto::{BlobHash, BundlePath};

const FORMAT_VERSION: u32 = 1;
use crate::orchestration::MANAGED_SKILL_MANIFEST as FILE;

#[derive(Debug, Serialize, Deserialize)]
struct ManagedFile {
    version: u32,
    #[serde(deserialize_with = "decode_files")]
    files: BTreeMap<BundlePath, BlobHash>,
}

fn decode_files<'de, D: Deserializer<'de>>(
    input: D,
) -> Result<BTreeMap<BundlePath, BlobHash>, D::Error> {
    struct Files;
    impl<'de> Visitor<'de> for Files {
        type Value = BTreeMap<BundlePath, BlobHash>;
        fn expecting(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(out, "at most {MAX_SKILL_FILES} managed files")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut source: A) -> Result<Self::Value, A::Error> {
            let mut files = BTreeMap::new();
            while let Some(path) = source.next_key::<BundlePath>()? {
                if files.len() == MAX_SKILL_FILES {
                    return Err(A::Error::custom("managed file count exceeds its bound"));
                }
                let hash = source.next_value()?;
                if files.insert(path, hash).is_some() {
                    return Err(A::Error::custom("duplicate managed path"));
                }
            }
            Ok(files)
        }
    }
    input.deserialize_map(Files)
}

pub(super) fn path() -> Result<BundlePath, SkillError> {
    Ok(FILE.parse()?)
}

pub(super) fn read(dir: &Dir) -> Result<Option<BTreeMap<BundlePath, BlobHash>>, SkillError> {
    let Some(bytes) = filesystem::read(dir, &path()?)? else {
        return Ok(None);
    };
    let parsed: ManagedFile = serde_json::from_slice(&bytes)?;
    if parsed.version != FORMAT_VERSION {
        return Err(SkillError::ManifestVersion);
    }
    if parsed.files.contains_key(&path()?) {
        return Err(SkillError::Blocked);
    }
    Ok(Some(parsed.files))
}

pub(super) fn write(dir: &Dir, bundle: &SkillBundle) -> Result<(), SkillError> {
    let files = bundle
        .files()
        .map(|asset| (asset.path().clone(), filesystem::hash(asset.bytes())))
        .collect();
    let bytes = serde_json::to_vec(&ManagedFile {
        version: FORMAT_VERSION,
        files,
    })?;
    filesystem::write(dir, &path()?, &bytes)
}
