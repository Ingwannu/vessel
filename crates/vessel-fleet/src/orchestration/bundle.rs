//! Immutable embedded bundle input, separate from installed mutable directories.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::SkillError;
use vessel_proto::BundlePath;
use vessel_proto::skills::{MAX_SKILLS, SkillName, SkillNames};

mod shipped {
    include!(concat!(env!("OUT_DIR"), "/skills.rs"));
}

/// Maximum individual file bytes before any home is changed.
pub(super) const MAX_SKILL_FILE_BYTES: usize = 1024 * 1024;
/// Maximum files in one bundled skill including its declaration.
pub(super) const MAX_SKILL_FILES: usize = 256;
/// Maximum total compiled skill bytes, bounding transaction preparation too.
pub(super) const MAX_SKILL_BUNDLE_BYTES: usize = 16 * 1024 * 1024;

/// One immutable file; content is shared between plans instead of cloned per target.
#[derive(Debug, Clone)]
pub struct SkillAsset {
    path: BundlePath,
    bytes: Arc<[u8]>,
}
impl SkillAsset {
    /// Validate the path and byte budget at the bundle ingestion edge.
    ///
    /// # Errors
    /// A path or file byte budget is invalid.
    pub fn new(path: BundlePath, bytes: Arc<[u8]>) -> Result<Self, SkillError> {
        if bytes.len() > MAX_SKILL_FILE_BYTES {
            return Err(SkillError::Budget);
        }
        Ok(Self { path, bytes })
    }
    /// Safe relative path inside the named skill directory.
    #[must_use]
    pub const fn path(&self) -> &BundlePath {
        &self.path
    }
    /// Shared compiled bytes; never user credentials or runtime configuration.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// One validated declaration and its files.
#[derive(Debug, Clone)]
pub struct SkillBundle {
    name: SkillName,
    files: BTreeMap<BundlePath, SkillAsset>,
}
impl SkillBundle {
    /// Validate a complete skill before it can be selected for installation.
    ///
    /// # Errors
    /// Too many files, duplicate paths or a missing/mismatched declaration.
    pub fn new(name: SkillName, assets: Vec<SkillAsset>) -> Result<Self, SkillError> {
        if assets.len() > MAX_SKILL_FILES {
            return Err(SkillError::Budget);
        }
        let mut files = BTreeMap::new();
        let mut portable = std::collections::HashSet::new();
        for asset in assets {
            if asset.path.as_str() == super::MANAGED_SKILL_MANIFEST
                || !portable.insert(crate::rel::fold(asset.path.as_str()))
            {
                return Err(SkillError::Duplicate);
            }
            if files.insert(asset.path.clone(), asset).is_some() {
                return Err(SkillError::Duplicate);
            }
        }
        let declaration = files
            .iter()
            .find(|(path, _)| path.as_str() == "SKILL.md")
            .and_then(|(_, asset)| std::str::from_utf8(asset.bytes()).ok())
            .and_then(|text| crate::skill_name(text).ok());
        if declaration.as_ref() != Some(&name) {
            return Err(SkillError::Declaration);
        }
        Ok(Self { name, files })
    }
    /// Stable directory identity from validated frontmatter.
    #[must_use]
    pub const fn name(&self) -> &SkillName {
        &self.name
    }
    /// Deterministic file order for status, update and ownership manifests.
    pub fn files(&self) -> impl Iterator<Item = &SkillAsset> {
        self.files.values()
    }
}

/// Full release catalogue; `all` uses this set rather than an independent list.
#[derive(Debug, Clone)]
pub struct SkillBundles(BTreeMap<SkillName, SkillBundle>);
impl SkillBundles {
    /// Construct bounded synthetic or packaged input without accessing a home.
    ///
    /// # Errors
    /// Duplicate skills or the total byte/skill budget is exceeded.
    pub fn new(bundles: Vec<SkillBundle>) -> Result<Self, SkillError> {
        if bundles.len() > MAX_SKILLS {
            return Err(SkillError::Budget);
        }
        let mut out = BTreeMap::new();
        let mut portable = std::collections::HashSet::new();
        let mut total = 0_usize;
        for bundle in bundles {
            if !portable.insert(crate::rel::fold(bundle.name.as_str())) {
                return Err(SkillError::Duplicate);
            }
            for asset in bundle.files() {
                total = total.saturating_add(asset.bytes().len());
            }
            if total > MAX_SKILL_BUNDLE_BYTES {
                return Err(SkillError::Budget);
            }
            if out.insert(bundle.name.clone(), bundle).is_some() {
                return Err(SkillError::Duplicate);
            }
        }
        Ok(Self(out))
    }
    /// The build's directory-derived catalogue, embedded into the binary.
    ///
    /// # Errors
    /// A shipped asset violates a path, declaration or resource invariant.
    pub fn shipped() -> Result<Self, SkillError> {
        let mut grouped: BTreeMap<SkillName, Vec<SkillAsset>> = BTreeMap::new();
        for (path, bytes) in shipped::FILES {
            let (name, relative) = path.split_once('/').ok_or(SkillError::Declaration)?;
            grouped
                .entry(name.parse()?)
                .or_default()
                .push(SkillAsset::new(relative.parse()?, Arc::from(*bytes))?);
        }
        let bundles = grouped
            .into_iter()
            .map(|(name, files)| SkillBundle::new(name, files))
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(bundles)
    }
    /// Sorted catalogue, including additions in a later release.
    ///
    /// # Errors
    /// The already bounded catalogue could not fit its wire name set.
    pub fn names(&self) -> Result<SkillNames, SkillError> {
        Ok(SkillNames::new(self.0.keys().cloned().collect())?)
    }
    /// Select exactly an existing bundled name; callers refuse unknown input.
    #[must_use]
    pub fn get(&self, name: &SkillName) -> Option<&SkillBundle> {
        self.0.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shipped_catalogue_uses_embedded_declarations() {
        let bundles = SkillBundles::shipped().unwrap();
        let names = bundles.names().unwrap();
        assert_eq!(names.as_slice().len(), 6);
        for name in names.as_slice() {
            let skill = bundles.get(name).unwrap();
            assert_eq!(skill.name(), name);
            assert!(skill.files().any(|file| file.path().as_str() == "SKILL.md"));
        }
    }
}
