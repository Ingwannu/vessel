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

fn service(root: &std::path::Path, bundles: SkillBundles) -> SkillService {
    SkillService {
        bundles,
        homes: SkillHomes {
            shared: SkillRoot::new(root.join("agents/skills")).unwrap(),
            claude: SkillRoot::new(root.join("claude/skills")).unwrap(),
            legacy_codex: SkillRoot::new(root.join("codex/skills")).unwrap(),
        },
    }
}

fn catalogue(version: &str, extra: Option<(&str, &[u8])>) -> SkillBundles {
    use crate::orchestration::{SkillAsset, SkillBundle};
    use std::sync::Arc;
    let text = format!("---\nname: vessel\n---\n{version}\n");
    let mut assets =
        vec![SkillAsset::new("SKILL.md".parse().unwrap(), Arc::from(text.as_bytes())).unwrap()];
    if let Some((path, bytes)) = extra {
        assets.push(SkillAsset::new(path.parse().unwrap(), Arc::from(bytes)).unwrap());
    }
    SkillBundles::new(vec![
        SkillBundle::new("vessel".parse().unwrap(), assets).unwrap(),
    ])
    .unwrap()
}

#[test]
fn startup_does_not_install_but_explicit_reconcile_installs_selected_files() {
    use vessel_proto::skills::{SkillSelection, SkillsState};
    let home = tempfile::tempdir().unwrap();
    let service = service(home.path(), catalogue("one", None));
    let fresh = service.maintain(&SkillSelection::All).unwrap();
    assert_eq!(fresh.state, SkillsState::NotInstalled);
    assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 0);
    let installed = service.reconcile(&SkillSelection::All).unwrap();
    assert_eq!(installed.state, SkillsState::UpToDate);
    for root in ["agents/skills", "claude/skills"] {
        assert!(home.path().join(root).join("vessel/SKILL.md").is_file());
    }
    assert!(!home.path().join("codex").exists());
}

#[test]
fn updates_keep_user_files_and_modified_obsolete_managed_files() {
    use vessel_proto::skills::{SkillNames, SkillSelection, SkillsState};
    let home = tempfile::tempdir().unwrap();
    let initial = service(home.path(), catalogue("one", Some(("old.md", b"owned"))));
    initial.reconcile(&SkillSelection::All).unwrap();
    let shared = home.path().join("agents/skills/vessel");
    std::fs::write(shared.join("old.md"), b"user edit").unwrap();
    std::fs::write(shared.join("notes.md"), b"user extra").unwrap();
    let updated = service(home.path(), catalogue("two", None));
    assert_eq!(
        updated.maintain(&SkillSelection::All).unwrap().state,
        SkillsState::UpToDate
    );
    assert_eq!(std::fs::read(shared.join("old.md")).unwrap(), b"user edit");
    assert_eq!(
        std::fs::read(shared.join("notes.md")).unwrap(),
        b"user extra"
    );
    assert!(!home.path().join("claude/skills/vessel/old.md").exists());
    let empty = SkillSelection::Custom {
        skills: SkillNames::default(),
    };
    let pending = updated.reconcile(&empty).unwrap();
    assert_eq!(pending.state, SkillsState::Drift);
    assert!(matches!(
        pending.ops.as_slice(),
        [vessel_proto::skills::SkillOperation::Delete { .. }]
    ));
    assert!(
        shared.join("notes.md").exists(),
        "reconcile must never delete an unselected directory"
    );
}

#[test]
fn unknown_selection_is_refused_before_creating_homes() {
    use vessel_proto::skills::{SkillNames, SkillSelection};
    let home = tempfile::tempdir().unwrap();
    let service = service(home.path(), catalogue("one", None));
    let selection = SkillSelection::Custom {
        skills: SkillNames::new(vec!["unknown".parse().unwrap()]).unwrap(),
    };
    assert!(matches!(
        service.reconcile(&selection),
        Err(SkillError::UnknownSkill)
    ));
    assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn update_preflights_every_asset_before_replacing_a_declaration() {
    use vessel_proto::skills::SkillSelection;
    let home = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let initial = service(home.path(), catalogue("one", None));
    initial.reconcile(&SkillSelection::All).unwrap();
    let shared = home.path().join("agents/skills/vessel");
    let original = std::fs::read(shared.join("SKILL.md")).unwrap();
    std::os::unix::fs::symlink(outside.path(), shared.join("sub")).unwrap();
    let updated = service(home.path(), catalogue("two", Some(("sub/file.md", b"new"))));
    assert!(updated.reconcile(&SkillSelection::All).is_err());
    assert_eq!(std::fs::read(shared.join("SKILL.md")).unwrap(), original);
    assert!(!outside.path().join("file.md").exists());
}
