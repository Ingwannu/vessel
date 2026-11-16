use super::*;
use crate::checkpoint::{GroupId, OrdinaryMode, OwnerId};

fn image(mode: u32, byte: u8) -> FileImage {
    FileImage::new(vec![Entry {
        path: CheckpointPath::try_from("file".parse::<crate::workspace::RelPath>().unwrap())
            .unwrap(),
        state: FileState::Present {
            digest: BlobHash::from_bytes([byte; 32]),
            bytes: CheckpointFileBytes::new(1).unwrap(),
            metadata: metadata(mode),
        },
    }])
    .unwrap()
}
fn metadata(mode: u32) -> FileMetadata {
    FileMetadata {
        owner: OwnerId::new(1000),
        group: GroupId::new(1000),
        mode: OrdinaryMode::new(mode).unwrap(),
    }
}
#[test]
fn effective_target_preserves_tightened_access_and_replays_only_execute_bits() {
    let captured = image(0o644, 1);
    let current = image(0o600, 2);
    assert_eq!(
        captured
            .effective(&current, metadata(0o600))
            .unwrap()
            .image(),
        &image(0o600, 1)
    );
    assert_eq!(
        image(0o755, 1)
            .effective(&current, metadata(0o600))
            .unwrap()
            .image(),
        &image(0o711, 1)
    );
}
#[test]
fn absent_recreation_is_private_and_equal_content_is_a_truthful_noop() {
    let empty = FileImage::new(Vec::new()).unwrap();
    let target = image(0o644, 1).effective(&empty, metadata(0o600)).unwrap();
    assert_eq!(target.image(), &image(0o600, 1));
    let current = image(0o600, 1);
    assert_eq!(
        image(0o644, 1)
            .effective(&current, metadata(0o600))
            .unwrap()
            .image(),
        &current
    );
}
#[test]
fn duplicate_paths_and_privileged_permissions_are_rejected() {
    let item = image(0o600, 1).entries().first().unwrap().clone();
    assert_eq!(
        FileImage::new(vec![item.clone(), item]),
        Err(CheckpointError::DuplicatePath)
    );
    assert_eq!(
        OrdinaryMode::new(0o4600),
        Err(CheckpointError::UnsupportedMetadata)
    );
}
#[test]
fn calls_stay_unassigned_without_any_fallback_to_conversation_rewind() {
    assert_eq!(crate::checkpoint::FILE_CHECKPOINTS, None);
}

#[test]
fn explicit_absence_matches_a_missing_file_and_rejects_later_unapproved_content() {
    let current = image(0o600, 1);
    let captured = FileImage::new(Vec::new()).unwrap();
    let desired = captured.effective(&current, metadata(0o600)).unwrap();
    assert!(desired.matches(&FileImage::new(Vec::new()).unwrap()));
    assert!(!desired.matches(&current));
}
