//! Durable observations must retain their invariants when decoded after restart.
use super::*;
fn path(name: &str) -> CheckpointPath {
    CheckpointPath::try_from(name.parse::<crate::workspace::RelPath>().unwrap()).unwrap()
}
fn evidence(entries: Vec<ObservedEntry>) -> SnapshotEvidence {
    SnapshotEvidence::new(
        RootFingerprint::new(BlobHash::from_bytes([1; 32])),
        IgnoreFingerprint::new(BlobHash::from_bytes([2; 32])),
        entries,
    )
    .unwrap()
}
#[test]
fn observations_are_canonical_and_duplicate_paths_refuse() {
    let first = ObservedEntry {
        path: path("a"),
        presence: ObservedPresence::Absent,
    };
    let second = ObservedEntry {
        path: path("b"),
        presence: ObservedPresence::Absent,
    };
    assert_eq!(
        evidence(vec![second.clone(), first.clone()]),
        evidence(vec![first.clone(), second])
    );
    let valid = evidence(vec![first.clone()]);
    assert_eq!(
        SnapshotEvidence::new(valid.root(), valid.ignore(), vec![first.clone(), first]),
        Err(CheckpointError::DuplicatePath)
    );
    let encoded = serde_json::to_string(&valid).unwrap();
    assert_eq!(
        serde_json::from_str::<SnapshotEvidence>(&encoded).unwrap(),
        valid
    );
}
#[test]
fn image_and_observations_must_cover_identical_paths_and_presence() {
    let image = FileImage::new(vec![super::super::Entry {
        path: path("a"),
        state: FileState::Absent,
    }])
    .unwrap();
    let omitted = evidence(Vec::new());
    assert_eq!(
        ObservedSnapshot::new(image.clone(), omitted),
        Err(CheckpointError::CaptureUnstable)
    );
    let different = evidence(vec![ObservedEntry {
        path: path("b"),
        presence: ObservedPresence::Absent,
    }]);
    assert_eq!(
        ObservedSnapshot::new(image.clone(), different),
        Err(CheckpointError::CaptureUnstable)
    );
    let present = evidence(vec![ObservedEntry {
        path: path("a"),
        presence: ObservedPresence::Present {
            full: FileFingerprint::new(BlobHash::from_bytes([3; 32])),
            object: ObjectFingerprint::new(BlobHash::from_bytes([4; 32])),
        },
    }]);
    assert_eq!(
        ObservedSnapshot::new(image.clone(), present),
        Err(CheckpointError::CaptureUnstable)
    );
    let valid = ObservedSnapshot::new(
        image,
        evidence(vec![ObservedEntry {
            path: path("a"),
            presence: ObservedPresence::Absent,
        }]),
    )
    .unwrap();
    let encoded = serde_json::to_string(&valid).unwrap();
    assert_eq!(
        serde_json::from_str::<ObservedSnapshot>(&encoded).unwrap(),
        valid
    );
}
#[test]
fn restart_decoder_cannot_bypass_the_content_evidence_association() {
    let invalid = r#"{"image":[],"evidence":{"root":"1111111111111111111111111111111111111111111111111111111111111111","ignore":"2222222222222222222222222222222222222222222222222222222222222222","entries":[{"path":"extra","presence":{"kind":"absent"}}]}}"#;
    assert!(serde_json::from_str::<ObservedSnapshot>(invalid).is_err());
}
