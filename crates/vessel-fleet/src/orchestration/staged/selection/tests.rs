//! Synthetic homes exercise failure paths independently of a node or provider.
use super::*;
use crate::orchestration::staged::tests::{catalogue, service};

#[test]
fn confirmation_leaves_directories_and_preference_untouched() {
    let temp = tempfile::tempdir().unwrap();
    let mut service = service(temp.path(), catalogue("one", None));
    let _installed = service.reconcile(&SkillSelection::All).unwrap();
    let state = SkillRoot::new(temp.path().join("transactions")).unwrap();
    let next = SkillSelection::Custom {
        skills: SkillNames::default(),
    };
    let result = service
        .save(
            &state,
            None,
            &next,
            &SkillNames::default(),
            |_next| -> Result<(), std::io::Error> { panic!("unconfirmed save persisted") },
        )
        .unwrap();
    assert!(result.confirmation_required.is_some());
    assert!(!temp.path().join("transactions").exists());
    assert!(temp.path().join("agents/skills/vessel/SKILL.md").exists());
    let result = service
        .save(
            &state,
            None,
            &next,
            &result.confirmation_required.unwrap(),
            |_next| Ok::<_, std::io::Error>(()),
        )
        .unwrap();
    assert!(result.snapshot.ops.is_empty());
    assert!(!temp.path().join("agents/skills/vessel").exists());
}

#[test]
fn failed_preference_write_restores_overwritten_and_obsolete_files() {
    let temp = tempfile::tempdir().unwrap();
    let mut old = service(temp.path(), catalogue("one", Some(("old.md", b"owned"))));
    let _installed = old.reconcile(&SkillSelection::All).unwrap();
    let shared = temp.path().join("agents/skills/vessel");
    std::fs::write(shared.join("user.md"), b"user").unwrap();
    let original = std::fs::read(shared.join("SKILL.md")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(
            shared.join("SKILL.md"),
            std::fs::Permissions::from_mode(0o750),
        )
        .unwrap();
    }
    let mut new = service(
        temp.path(),
        catalogue("two", Some(("new/nested.md", b"new"))),
    );
    let state = SkillRoot::new(temp.path().join("transactions")).unwrap();
    let result = new.save(
        &state,
        None,
        &SkillSelection::All,
        &SkillNames::default(),
        |_next| Err(std::io::Error::other("fake persistence failure")),
    );
    assert!(matches!(
        result,
        Err(SelectionError::Persist { rollback: None, .. })
    ));
    assert_eq!(std::fs::read(shared.join("SKILL.md")).unwrap(), original);
    assert_eq!(std::fs::read(shared.join("old.md")).unwrap(), b"owned");
    assert_eq!(std::fs::read(shared.join("user.md")).unwrap(), b"user");
    assert!(!shared.join("new/nested.md").exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(
            std::fs::metadata(shared.join("SKILL.md"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o750
        );
    }
    assert_eq!(
        std::fs::read_dir(temp.path().join("transactions"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn outside_edit_survives_rollback_and_keeps_its_backup() {
    let temp = tempfile::tempdir().unwrap();
    let mut old = service(temp.path(), catalogue("one", None));
    let _installed = old.reconcile(&SkillSelection::All).unwrap();
    let shared = temp.path().join("agents/skills/vessel/SKILL.md");
    let mut new = service(temp.path(), catalogue("two", None));
    let state = SkillRoot::new(temp.path().join("transactions")).unwrap();
    let result = new.save(
        &state,
        None,
        &SkillSelection::All,
        &SkillNames::default(),
        |_next| {
            std::fs::write(&shared, b"outside edit").unwrap();
            Err(std::io::Error::other("fake persistence failure"))
        },
    );
    assert!(matches!(
        result,
        Err(SelectionError::Persist {
            rollback: Some(_),
            ..
        })
    ));
    assert_eq!(std::fs::read(&shared).unwrap(), b"outside edit");
    assert_eq!(
        std::fs::read_dir(temp.path().join("transactions"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn interrupted_patch_application_recovers_with_only_the_old_durable_choice() {
    let temp = tempfile::tempdir().unwrap();
    let mut old = service(temp.path(), catalogue("one", Some(("old.md", b"old"))));
    let _installed = old.reconcile(&SkillSelection::All).unwrap();
    let shared = temp.path().join("agents/skills/vessel");
    let original = std::fs::read(shared.join("SKILL.md")).unwrap();
    let mut new = service(
        temp.path(),
        catalogue("two", Some(("nested/new.md", b"new"))),
    );
    let plan = new.status(&SkillSelection::All).unwrap();
    let state = SkillRoot::new(temp.path().join("transactions")).unwrap();
    let mut transaction =
        deletion::DeletionTransaction::begin(&new.homes, &state, None, SkillSelection::All, &plan)
            .unwrap();
    transaction.capture_patches(&new, &plan).unwrap();
    transaction.apply().unwrap();
    new.apply_updates(&plan).unwrap();
    assert_ne!(std::fs::read(shared.join("SKILL.md")).unwrap(), original);
    drop(transaction); // Process-like interruption before durable preference save.
    let mut restarted = service(temp.path(), catalogue("two", None));
    restarted.recover(&state, None).unwrap();
    assert_eq!(std::fs::read(shared.join("SKILL.md")).unwrap(), original);
    assert_eq!(std::fs::read(shared.join("old.md")).unwrap(), b"old");
    assert!(!shared.join("nested/new.md").exists());
    assert_eq!(
        std::fs::read_dir(temp.path().join("transactions"))
            .unwrap()
            .count(),
        0
    );
}
