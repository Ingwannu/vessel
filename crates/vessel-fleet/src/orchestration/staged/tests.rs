//! All skill service tests keep explicit synthetic homes below a temporary root.

use super::*;
use crate::dir::Entry;
use crate::orchestration::staged::filesystem::{self, hash};

#[test]
fn safe_file_operations_preserve_changed_obsolete_files() {
    let home = tempfile::tempdir().unwrap();
    let root = SkillRoot::new(home.path().join("skills")).unwrap();
    assert!(root.read().unwrap().is_none());
    let dir = root.create().unwrap();
    let path = "nested/file.md".parse().unwrap();
    filesystem::write(&dir, &path, b"old").unwrap();
    filesystem::remove_unchanged(&dir, &path, hash(b"different")).unwrap();
    assert_eq!(filesystem::read(&dir, &path).unwrap().unwrap(), b"old");
    filesystem::remove_unchanged(&dir, &path, hash(b"old")).unwrap();
    assert!(filesystem::read(&dir, &path).unwrap().is_none());
    filesystem::remove_directory(&dir, "nested").unwrap();
    assert_eq!(dir.entry("nested").unwrap(), Entry::Missing);
}

#[cfg(unix)]
#[test]
fn a_replaced_child_directory_cannot_redirect_a_write() {
    let home = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let root = SkillRoot::new(home.path().to_path_buf()).unwrap();
    let dir = root.read().unwrap().unwrap();
    std::os::unix::fs::symlink(outside.path(), home.path().join("nested")).unwrap();
    let path = "nested/file.md".parse().unwrap();
    assert!(filesystem::write(&dir, &path, b"forbidden").is_err());
    assert!(!outside.path().join("file.md").exists());
}

#[test]
fn directory_staging_stays_beside_the_held_root() {
    let home = tempfile::tempdir().unwrap();
    let root = SkillRoot::new(home.path().to_path_buf())
        .unwrap()
        .read()
        .unwrap()
        .unwrap();
    let _skill = root.create_dir("skill").unwrap();
    root.move_entry("skill", ".stage").unwrap();
    assert_eq!(root.entry("skill").unwrap(), Entry::Missing);
    root.move_entry(".stage", "skill").unwrap();
    filesystem::remove_directory(&root, "skill").unwrap();
}

#[test]
fn staged_service_captures_only_explicit_roots_and_the_shipped_catalogue() {
    let home = tempfile::tempdir().unwrap();
    let service = SkillService {
        bundles: SkillBundles::shipped().unwrap(),
        homes: SkillHomes {
            shared: SkillRoot::new(home.path().join("agents/skills")).unwrap(),
            claude: SkillRoot::new(home.path().join("claude/skills")).unwrap(),
            legacy_codex: SkillRoot::new(home.path().join("codex/skills")).unwrap(),
        },
    };
    assert!(!service.bundles.names().unwrap().is_empty());
    for root in service.homes.roots() {
        assert!(root.read().unwrap().is_none());
    }
    assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 0);
}
