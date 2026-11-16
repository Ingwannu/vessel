use std::error::Error as _;

use vessel_proto::parking::ParkAfter;

use super::*;

#[test]
fn one_snapshot_binds_the_original_document_and_its_coverage() {
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path());
    let original = br#"{"park_after":{"kind":"never"},"later":{"keep":"original"}}"#;
    std::fs::write(data.settings_path(), original).unwrap();
    let snapshot = data.read_settings_snapshot().unwrap();
    std::fs::write(data.settings_path(), b"{}").unwrap();
    let (saved, disposition) = snapshot.into_parts();
    assert_eq!(
        saved.unwrap().public_settings().park_after,
        Some(ParkAfter::Never)
    );
    assert_eq!(disposition, SettingsDisposition::PreserveBeforeWrite);
    let (_, replacement) = data.read_settings_snapshot().unwrap().into_parts();
    assert_eq!(replacement, SettingsDisposition::Current);
}

#[test]
fn missing_null_and_canonical_documents_keep_the_existing_semantics() {
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path());
    let (missing, disposition) = data.read_settings_snapshot().unwrap().into_parts();
    assert!(missing.is_none());
    assert_eq!(disposition, SettingsDisposition::Current);
    for input in [b"{}".as_slice(), br#"{"park_after":null,"drivers":null}"#] {
        std::fs::write(data.settings_path(), input).unwrap();
        let (snapshot, disposition) = data.read_settings_snapshot().unwrap().into_parts();
        let legacy = data.read_settings().unwrap();
        assert_eq!(snapshot, legacy);
        let known = snapshot.unwrap();
        assert_eq!(
            matches!(disposition, SettingsDisposition::PreserveBeforeWrite),
            data.settings_hold_more_than(&known).unwrap()
        );
    }
}

#[test]
fn snapshot_diagnostics_never_reveal_the_private_concrete_parse_cause() {
    const MARKER: &str = "private-snapshot-marker-62e8";
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path());
    let bytes = format!(r#"{{"drivers":"{MARKER}"}}"#);
    std::fs::write(data.settings_path(), &bytes).unwrap();
    let error = data.read_settings_snapshot().unwrap_err();
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(&error);
    while let Some(error) = source {
        assert!(!error.to_string().contains(MARKER));
        assert!(!format!("{error:?}").contains(MARKER));
        source = error.source();
    }
    assert!(error.to_string().contains("invalid field at $.<field>"));
    assert_eq!(
        std::fs::read(data.settings_path()).unwrap(),
        bytes.as_bytes()
    );
    assert!(error.source().unwrap().source().is_none());
    // The legacy coverage query compares JSON with a separately known model;
    // it is not a second typed validator. Preserve its conservative result.
    assert!(
        data.settings_hold_more_than(&SavedNodeSettings::default())
            .unwrap()
    );
    // Exercise an actual compatibility-parser failure independently of the
    // canonical typed failure, retaining the same private-value canary.
    let malformed = format!(r#"{{"drivers":"{MARKER}""#);
    std::fs::write(data.settings_path(), &malformed).unwrap();
    let compatibility = data
        .settings_hold_more_than(&SavedNodeSettings::default())
        .unwrap_err();
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(&compatibility);
    while let Some(error) = current {
        assert!(!error.to_string().contains(MARKER));
        assert!(!format!("{error:?}").contains(MARKER));
        current = error.source();
    }
    assert_eq!(
        std::fs::read(data.settings_path()).unwrap(),
        malformed.as_bytes()
    );
}
