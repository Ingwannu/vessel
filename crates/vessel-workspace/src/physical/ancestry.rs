//! Walk opened parents so aliases and nested CWDs use the managed root's gate.
use super::{Directory, key};
use std::{collections::BTreeSet, fs::File, io};
use vessel_proto::checkpoint::PhysicalRoot;

/// Refuse an unprovable/cyclic ancestry instead of starting outside admission.
const MAX_ANCESTORS: usize = 4096;
/// Physical parents in nearest-first order; logical workspace ids never appear.
#[derive(Debug)]
pub struct Ancestry {
    root: PhysicalRoot,
    parents: Vec<PhysicalRoot>,
}
impl Ancestry {
    /// The opened object at the beginning of this observation, not a path key.
    #[must_use]
    pub const fn root(&self) -> PhysicalRoot {
        self.root
    }
    /// Nested scopes conflict in either order; shared parents alone do not.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.root == other.root
            || self.parents.contains(&other.root)
            || other.parents.contains(&self.root)
    }
    /// Find the nearest registered root, including a parent alias.
    #[must_use]
    pub fn nearest(&self, known: &BTreeSet<PhysicalRoot>) -> Option<PhysicalRoot> {
        std::iter::once(self.root)
            .chain(self.parents.iter().copied())
            .find(|root| known.contains(root))
    }
}
impl Directory {
    /// Inspect parents through FDs rather than comparing caller-controlled paths.
    ///
    /// # Errors
    /// Inaccessible, cyclic or excessively deep ancestry refuses the start.
    pub fn ancestry(&self) -> io::Result<Ancestry> {
        let mut current = self.file.try_clone()?;
        let mut identities = Vec::new();
        let mut seen = BTreeSet::new();
        for _ in 0..MAX_ANCESTORS {
            let identity = key(&current.metadata()?);
            if !seen.insert(identity) {
                return Err(io::Error::other("physical ancestry is cyclic"));
            }
            identities.push(identity);
            let parent = File::from(
                nix::fcntl::openat(
                    &current,
                    "..",
                    nix::fcntl::OFlag::O_RDONLY
                        | nix::fcntl::OFlag::O_DIRECTORY
                        | nix::fcntl::OFlag::O_NOFOLLOW
                        | nix::fcntl::OFlag::O_CLOEXEC,
                    nix::sys::stat::Mode::empty(),
                )
                .map_err(io::Error::from)?,
            );
            if key(&parent.metadata()?) == identity {
                return Ok(Ancestry {
                    root: self.identity(),
                    parents: identities.into_iter().skip(1).collect(),
                });
            }
            current = parent;
        }
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "physical ancestry is too deep",
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checkpoint_nested_cwd_and_parent_alias_find_the_same_physical_root() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        std::fs::create_dir_all(root.join("nested/deeper")).unwrap();
        let directory = Directory::open(&root).unwrap();
        let nested = Directory::open(&root.join("nested/deeper")).unwrap();
        let known = BTreeSet::from([directory.identity()]);
        assert_eq!(
            nested.ancestry().unwrap().nearest(&known),
            Some(directory.identity())
        );
        let alias = temp.path().join("alias");
        std::os::unix::fs::symlink(&root, &alias).unwrap();
        let aliased = Directory::open(&alias.join("nested")).unwrap();
        assert_eq!(
            aliased.ancestry().unwrap().nearest(&known),
            Some(directory.identity())
        );
        assert_eq!(
            Directory::open(temp.path())
                .unwrap()
                .ancestry()
                .unwrap()
                .nearest(&known),
            None
        );
    }
}
