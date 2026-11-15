//! Walk opened parents so aliases and nested CWDs use the managed root's gate.
use super::{Directory, key};
use std::{collections::BTreeSet, fs::File, io};
use vessel_proto::checkpoint::PhysicalRoot;

/// Refuse an unprovable/cyclic ancestry instead of starting outside admission.
const MAX_ANCESTORS: usize = 4096;
/// Physical parents in nearest-first order; logical workspace ids never appear.
#[derive(Debug)]
pub struct Ancestry(Vec<PhysicalRoot>);
impl Ancestry {
    /// Find the nearest managed root, including a bind or symlink parent alias.
    #[must_use]
    pub fn nearest(&self, known: &BTreeSet<PhysicalRoot>) -> Option<PhysicalRoot> {
        self.0.iter().copied().find(|root| known.contains(root))
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
                return Ok(Ancestry(identities));
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
