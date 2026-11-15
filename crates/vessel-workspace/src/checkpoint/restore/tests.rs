//! Permission policy and guarded rollback are tested against real temporary files.
use super::{
    Prepared, Undo,
    stage::{Slot, SlotKind, Stage},
};
use crate::{
    checkpoint::{archive::Archive, capture, enumeration, inspect, rooted::Root},
    testing::Sandbox,
};
use std::{
    fs,
    os::unix::fs::{MetadataExt as _, PermissionsExt as _},
};
use tokio_util::sync::CancellationToken;
use vessel_proto::checkpoint::{
    CheckpointError, FileMetadata, FileState, GroupId, OrdinaryMode, OwnerId, PathCount,
};

fn private_dir(sandbox: &Sandbox, name: &str) -> std::path::PathBuf {
    let path = sandbox.path().join(name);
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}
fn account(root: &std::path::Path) -> FileMetadata {
    let found = fs::metadata(root).unwrap();
    FileMetadata {
        owner: OwnerId::new(found.uid()),
        group: GroupId::new(found.gid()),
        mode: OrdinaryMode::new(0o600).unwrap(),
    }
}

#[tokio::test]
async fn checkpoint_restart_reconstructs_staged_objects_and_rolls_back_without_forward_replay() {
    use super::journal::RecoveryState;
    let sandbox = Sandbox::new();
    let dir = sandbox.repo("project");
    let git = sandbox.git().await;
    let cancel = CancellationToken::new();
    let archive = Archive::open(private_dir(&sandbox, "captured")).unwrap();
    let paths = enumeration::paths(&git, &dir).await.unwrap();
    let selected = capture::image(&Root::open(&dir).unwrap(), &archive, &paths, &cancel).unwrap();
    fs::write(dir.join("README.md"), b"later edit").unwrap();
    let before_archive = Archive::open(private_dir(&sandbox, "before")).unwrap();
    let before =
        capture::image(&Root::open(&dir).unwrap(), &before_archive, &paths, &cancel).unwrap();
    let target = selected.effective(&before, account(&dir)).unwrap();
    let stage_path = private_dir(&sandbox, "journal");
    let stage = Stage::open(&stage_path).unwrap();
    let root = Root::open(&dir).unwrap();
    let prepared = Prepared::new(&root, &archive, &stage, &before, &target, &cancel).unwrap();
    let encoded = serde_json::to_string(&prepared.evidence().unwrap()).unwrap();
    let index = PathCount::new(0).unwrap();
    assert_eq!(
        prepared.recovery_state(&root, index, &cancel).unwrap(),
        RecoveryState::Original
    );
    prepared.apply(&root, &stage, index, &cancel).unwrap();
    drop(prepared);
    drop(stage);
    drop(root);
    let stage = Stage::open(&stage_path).unwrap();
    let root = Root::open(&dir).unwrap();
    let recovered =
        Prepared::recovering(&before, &target, &serde_json::from_str(&encoded).unwrap()).unwrap();
    assert_eq!(
        recovered.recovery_state(&root, index, &cancel).unwrap(),
        RecoveryState::Desired
    );
    assert!(recovered.apply(&root, &stage, index, &cancel).is_err());
    assert_eq!(
        recovered.undo(&root, &stage, index, &cancel).unwrap(),
        Undo::Restored
    );
    assert_eq!(
        recovered.recovery_state(&root, index, &cancel).unwrap(),
        RecoveryState::Original
    );
    assert_eq!(fs::read(dir.join("README.md")).unwrap(), b"later edit");
}

#[tokio::test]
async fn checkpoint_restart_keeps_an_equal_content_external_replacement_as_a_conflict() {
    use super::journal::RecoveryState;
    let sandbox = Sandbox::new();
    let dir = sandbox.repo("project");
    let git = sandbox.git().await;
    let cancel = CancellationToken::new();
    let archive = Archive::open(private_dir(&sandbox, "captured")).unwrap();
    let paths = enumeration::paths(&git, &dir).await.unwrap();
    let selected = capture::image(&Root::open(&dir).unwrap(), &archive, &paths, &cancel).unwrap();
    fs::write(dir.join("README.md"), b"later edit").unwrap();
    let before_archive = Archive::open(private_dir(&sandbox, "before")).unwrap();
    let before =
        capture::image(&Root::open(&dir).unwrap(), &before_archive, &paths, &cancel).unwrap();
    let target = selected.effective(&before, account(&dir)).unwrap();
    let stage = Stage::open(&private_dir(&sandbox, "journal")).unwrap();
    let root = Root::open(&dir).unwrap();
    let prepared = Prepared::new(&root, &archive, &stage, &before, &target, &cancel).unwrap();
    let encoded = serde_json::to_string(&prepared.evidence().unwrap()).unwrap();
    let index = PathCount::new(0).unwrap();
    prepared.apply(&root, &stage, index, &cancel).unwrap();
    let current_mode = fs::metadata(dir.join("README.md")).unwrap().permissions();
    fs::write(
        dir.join("outside"),
        fs::read(dir.join("README.md")).unwrap(),
    )
    .unwrap();
    fs::set_permissions(dir.join("outside"), current_mode).unwrap();
    fs::rename(dir.join("outside"), dir.join("README.md")).unwrap();
    drop(prepared);
    let recovered =
        Prepared::recovering(&before, &target, &serde_json::from_str(&encoded).unwrap()).unwrap();
    assert_eq!(
        recovered.recovery_state(&root, index, &cancel).unwrap(),
        RecoveryState::Conflict
    );
    assert_eq!(
        recovered.undo(&root, &stage, index, &cancel).unwrap(),
        Undo::Conflict
    );
    assert_eq!(fs::read(dir.join("README.md")).unwrap(), b"hello\n");
}
#[tokio::test]
async fn restore_retains_current_read_write_bits_and_rollback_restores_the_actual_original() {
    let sandbox = Sandbox::new();
    let dir = sandbox.repo("project");
    assert_ne!(
        fs::metadata(&dir).unwrap().uid(),
        0,
        "permission proof must run as a non-root account"
    );
    let git = sandbox.git().await;
    let cancel = CancellationToken::new();
    let archive = Archive::open(private_dir(&sandbox, "captured")).unwrap();
    fs::set_permissions(dir.join("README.md"), fs::Permissions::from_mode(0o644)).unwrap();
    let paths = enumeration::paths(&git, &dir).await.unwrap();
    let root = Root::open(&dir).unwrap();
    let selected = capture::image(&root, &archive, &paths, &cancel).unwrap();
    fs::write(dir.join("README.md"), b"later edit").unwrap();
    fs::set_permissions(dir.join("README.md"), fs::Permissions::from_mode(0o600)).unwrap();
    let before_archive = Archive::open(private_dir(&sandbox, "before")).unwrap();
    let before =
        capture::image(&Root::open(&dir).unwrap(), &before_archive, &paths, &cancel).unwrap();
    let target = selected.effective(&before, account(&dir)).unwrap();
    let stage = Stage::open(&private_dir(&sandbox, "journal")).unwrap();
    let root = Root::open(&dir).unwrap();
    let prepared = Prepared::new(&root, &archive, &stage, &before, &target, &cancel).unwrap();
    assert_eq!(prepared.steps().len(), 1);
    let index = PathCount::new(0).unwrap();
    prepared.apply(&root, &stage, index, &cancel).unwrap();
    assert_eq!(fs::read(dir.join("README.md")).unwrap(), b"hello\n");
    assert_eq!(
        fs::metadata(dir.join("README.md")).unwrap().mode() & 0o777,
        0o600
    );
    assert_eq!(
        prepared.undo(&root, &stage, index, &cancel).unwrap(),
        Undo::Restored
    );
    assert_eq!(fs::read(dir.join("README.md")).unwrap(), b"later edit");
    stage.remove(Slot::new(index, SlotKind::Desired)).unwrap();
}

#[tokio::test]
async fn executable_only_restore_replays_execute_bits_and_preserves_current_access_bits() {
    let sandbox = Sandbox::new();
    let dir = sandbox.repo("project");
    let git = sandbox.git().await;
    let cancel = CancellationToken::new();
    let archive = Archive::open(private_dir(&sandbox, "captured")).unwrap();
    let file = dir.join("README.md");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
    let paths = enumeration::paths(&git, &dir).await.unwrap();
    let selected = capture::image(&Root::open(&dir).unwrap(), &archive, &paths, &cancel).unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
    let before_archive = Archive::open(private_dir(&sandbox, "before")).unwrap();
    let before =
        capture::image(&Root::open(&dir).unwrap(), &before_archive, &paths, &cancel).unwrap();
    let target = selected.effective(&before, account(&dir)).unwrap();
    assert_eq!(
        target.changes(&before).first().unwrap().kind,
        vessel_proto::checkpoint::ChangeKind::Permissions
    );
    let stage = Stage::open(&private_dir(&sandbox, "journal")).unwrap();
    let root = Root::open(&dir).unwrap();
    let prepared = Prepared::new(&root, &archive, &stage, &before, &target, &cancel).unwrap();
    let index = PathCount::new(0).unwrap();
    prepared.apply(&root, &stage, index, &cancel).unwrap();
    assert_eq!(fs::metadata(&file).unwrap().mode() & 0o777, 0o711);
    assert_eq!(fs::read(&file).unwrap(), b"hello\n");
    assert_eq!(
        prepared.undo(&root, &stage, index, &cancel).unwrap(),
        Undo::Restored
    );
    assert_eq!(fs::metadata(&file).unwrap().mode() & 0o777, 0o600);
}

#[tokio::test]
async fn later_created_file_deletion_can_be_reversed_from_its_retained_inode() {
    let sandbox = Sandbox::new();
    let dir = sandbox.repo("project");
    let git = sandbox.git().await;
    let cancel = CancellationToken::new();
    let archive = Archive::open(private_dir(&sandbox, "captured")).unwrap();
    let selected = capture::image(
        &Root::open(&dir).unwrap(),
        &archive,
        &enumeration::paths(&git, &dir).await.unwrap(),
        &cancel,
    )
    .unwrap();
    fs::write(dir.join("later"), b"preserve me during rollback").unwrap();
    let before_archive = Archive::open(private_dir(&sandbox, "before")).unwrap();
    let paths = enumeration::paths(&git, &dir).await.unwrap();
    let before =
        capture::image(&Root::open(&dir).unwrap(), &before_archive, &paths, &cancel).unwrap();
    let target = selected.effective(&before, account(&dir)).unwrap();
    let stage = Stage::open(&private_dir(&sandbox, "journal")).unwrap();
    let root = Root::open(&dir).unwrap();
    let prepared = Prepared::new(&root, &archive, &stage, &before, &target, &cancel).unwrap();
    assert_eq!(prepared.steps().len(), 1);
    let index = PathCount::new(0).unwrap();
    prepared.apply(&root, &stage, index, &cancel).unwrap();
    assert!(!dir.join("later").exists());
    assert_eq!(
        prepared.undo(&root, &stage, index, &cancel).unwrap(),
        Undo::Restored
    );
    assert_eq!(
        fs::read(dir.join("later")).unwrap(),
        b"preserve me during rollback"
    );
}

#[tokio::test]
async fn equal_bytes_at_a_replaced_inode_do_not_authorize_apply_or_rollback() {
    let sandbox = Sandbox::new();
    let dir = sandbox.repo("project");
    let git = sandbox.git().await;
    let cancel = CancellationToken::new();
    let archive = Archive::open(private_dir(&sandbox, "captured")).unwrap();
    let paths = enumeration::paths(&git, &dir).await.unwrap();
    let selected = capture::image(&Root::open(&dir).unwrap(), &archive, &paths, &cancel).unwrap();
    let file = dir.join("README.md");
    fs::write(&file, b"before restore").unwrap();
    let before_archive = Archive::open(private_dir(&sandbox, "before")).unwrap();
    let before =
        capture::image(&Root::open(&dir).unwrap(), &before_archive, &paths, &cancel).unwrap();
    let target = selected.effective(&before, account(&dir)).unwrap();
    let root = Root::open(&dir).unwrap();
    let stage = Stage::open(&private_dir(&sandbox, "journal")).unwrap();
    let prepared = Prepared::new(&root, &archive, &stage, &before, &target, &cancel).unwrap();
    let replacement = dir.join("replacement");
    let _copied = fs::copy(&file, &replacement).unwrap();
    fs::rename(&replacement, &file).unwrap();
    let index = PathCount::new(0).unwrap();
    assert!(matches!(
        prepared.apply(&root, &stage, index, &cancel),
        Err(crate::checkpoint::Error::Refused(
            CheckpointError::StalePreview
        ))
    ));
    let stage = Stage::open(&private_dir(&sandbox, "journal-second")).unwrap();
    let prepared = Prepared::new(&root, &archive, &stage, &before, &target, &cancel).unwrap();
    prepared.apply(&root, &stage, index, &cancel).unwrap();
    let _copied = fs::copy(&file, &replacement).unwrap();
    fs::rename(&replacement, &file).unwrap();
    assert_eq!(
        prepared.undo(&root, &stage, index, &cancel).unwrap(),
        Undo::Conflict
    );
    assert_eq!(fs::read(&file).unwrap(), b"hello\n");
}
#[tokio::test]
async fn rollback_refuses_to_overwrite_a_new_external_edit() {
    let sandbox = Sandbox::new();
    let dir = sandbox.repo("project");
    let git = sandbox.git().await;
    let cancel = CancellationToken::new();
    let archive = Archive::open(private_dir(&sandbox, "captured")).unwrap();
    let paths = enumeration::paths(&git, &dir).await.unwrap();
    let selected = capture::image(&Root::open(&dir).unwrap(), &archive, &paths, &cancel).unwrap();
    fs::write(dir.join("README.md"), b"before restore").unwrap();
    let before_archive = Archive::open(private_dir(&sandbox, "before")).unwrap();
    let before =
        capture::image(&Root::open(&dir).unwrap(), &before_archive, &paths, &cancel).unwrap();
    let target = selected.effective(&before, account(&dir)).unwrap();
    let stage = Stage::open(&private_dir(&sandbox, "journal")).unwrap();
    let root = Root::open(&dir).unwrap();
    let prepared = Prepared::new(&root, &archive, &stage, &before, &target, &cancel).unwrap();
    let index = PathCount::new(0).unwrap();
    prepared.apply(&root, &stage, index, &cancel).unwrap();
    fs::write(dir.join("README.md"), b"external after apply").unwrap();
    assert_eq!(
        prepared.undo(&root, &stage, index, &cancel).unwrap(),
        Undo::Conflict
    );
    assert_eq!(
        fs::read(dir.join("README.md")).unwrap(),
        b"external after apply"
    );
}
#[tokio::test]
async fn stale_preflight_and_absent_recreation_preserve_policy_and_tell_the_truth() {
    let sandbox = Sandbox::new();
    let dir = sandbox.repo("project");
    let git = sandbox.git().await;
    let cancel = CancellationToken::new();
    let archive = Archive::open(private_dir(&sandbox, "captured")).unwrap();
    let paths = enumeration::paths(&git, &dir).await.unwrap();
    let selected = capture::image(&Root::open(&dir).unwrap(), &archive, &paths, &cancel).unwrap();
    fs::remove_file(dir.join("README.md")).unwrap();
    let before_archive = Archive::open(private_dir(&sandbox, "before")).unwrap();
    let before =
        capture::image(&Root::open(&dir).unwrap(), &before_archive, &paths, &cancel).unwrap();
    let target = selected.effective(&before, account(&dir)).unwrap();
    let stage = Stage::open(&private_dir(&sandbox, "journal")).unwrap();
    let root = Root::open(&dir).unwrap();
    let prepared = Prepared::new(&root, &archive, &stage, &before, &target, &cancel).unwrap();
    let index = PathCount::new(0).unwrap();
    prepared.apply(&root, &stage, index, &cancel).unwrap();
    let restored = inspect::path(&root, &prepared.steps().first().unwrap().path, &cancel).unwrap();
    let FileState::Present { metadata, .. } = restored else {
        panic!("restored file missing")
    };
    assert_eq!(metadata.mode.get(), 0o600);
    assert!(matches!(
        prepared.apply(&root, &stage, index, &cancel),
        Err(crate::checkpoint::Error::Refused(
            CheckpointError::StalePreview
        ))
    ));
    assert_eq!(
        prepared.undo(&root, &stage, index, &cancel).unwrap(),
        Undo::Restored
    );
    assert!(!dir.join("README.md").exists());
}
