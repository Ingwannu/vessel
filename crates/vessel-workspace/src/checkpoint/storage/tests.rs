//! Private names never loosen existing permissions or follow a substituted link.
use super::*;
use std::{
    fs,
    os::unix::fs::{MetadataExt as _, PermissionsExt as _, symlink},
};
use vessel_proto::{
    EventSeq, SessionId, WorktreeId,
    checkpoint::{Boundary, CaptureGeneration, CaptureId},
};

fn key() -> CaptureKey {
    CaptureKey {
        capture: CaptureId::random(),
        worktree: WorktreeId::random(),
        session: SessionId::random(),
        generation: CaptureGeneration::FIRST,
        boundary: Boundary::BeforeTurn {
            prompt: EventSeq::ZERO,
        },
    }
}
#[test]
fn checkpoint_storage_creates_private_non_reused_names_and_refuses_existing_links_or_access() {
    let data = tempfile::tempdir().unwrap();
    fs::set_permissions(data.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let storage = Storage::open(data.path()).unwrap();
    let key = key();
    let capture = storage.create_capture(key).unwrap();
    let _archive = capture.archive().unwrap();
    let plan = RestorePlanId::random();
    let _current = capture.create_preview(plan).unwrap();
    let _reopened = capture.open_preview(plan).unwrap();
    let restore = RestoreId::random();
    let stage = capture.create_restore(restore).unwrap();
    let _reopened = capture.open_restore(restore).unwrap();
    assert_eq!(fs::metadata(stage.path()).unwrap().mode() & 0o7777, 0o700);
    assert_eq!(
        fs::metadata(stage.path()).unwrap().uid(),
        rustix::process::geteuid().as_raw()
    );
    assert!(storage.create_capture(key).is_err());
    let _existing = storage.open_capture(key).unwrap();
    let foreign = tempfile::tempdir().unwrap();
    fs::set_permissions(foreign.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let alias = WorktreeId::random();
    symlink(foreign.path(), storage.folder.path.join(alias.to_string())).unwrap();
    let linked = CaptureKey {
        worktree: alias,
        ..key
    };
    assert!(storage.create_capture(linked).is_err());
    assert!(fs::read_dir(foreign.path()).unwrap().next().is_none());
    let broad = WorktreeId::random();
    let path = storage.folder.path.join(broad.to_string());
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o750)).unwrap();
    assert!(
        storage
            .create_capture(CaptureKey {
                worktree: broad,
                ..key
            })
            .is_err()
    );
    assert_eq!(
        fs::metadata(path).unwrap().mode() & 0o7777,
        0o750,
        "reuse never repairs or broadens an existing access policy"
    );
}

const CHILD_ROOT: &str = "VESSEL_TEST_CHECKPOINT_UMASK_ROOT";
#[test]
fn checkpoint_storage_directory_creation_is_independent_of_umask_in_an_isolated_child() {
    let data = tempfile::tempdir().unwrap();
    fs::set_permissions(data.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "checkpoint::storage::tests::checkpoint_storage_umask_child",
            "--test-threads=1",
        ])
        .env(CHILD_ROOT, data.path())
        .status()
        .unwrap();
    assert!(status.success());
    assert!(
        data.path().join("checkpoints").is_dir(),
        "the exact child fixture must actually execute"
    );
}
#[test]
#[ignore = "isolated subprocess helper, executed explicitly by the parent umask regression"]
fn checkpoint_storage_umask_child() {
    let Some(root) = std::env::var_os(CHILD_ROOT) else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    assert_ne!(
        rustix::process::geteuid().as_raw(),
        0,
        "permission fixture must run as the non-root build account"
    );
    // Only this one-test child changes its umask; never the parent test process.
    let _previous = rustix::process::umask(rustix::fs::Mode::from_raw_mode(0o777));
    let storage = Storage::open(&root).unwrap();
    let capture = storage.create_capture(key()).unwrap();
    let _archive = capture.archive().unwrap();
    let _preview = capture.create_preview(RestorePlanId::random()).unwrap();
    let stage = capture.create_restore(RestoreId::random()).unwrap();
    assert_eq!(fs::metadata(stage.path()).unwrap().mode() & 0o7777, 0o700);
    assert_eq!(
        fs::metadata(root.join("checkpoints")).unwrap().mode() & 0o7777,
        0o700
    );
}
