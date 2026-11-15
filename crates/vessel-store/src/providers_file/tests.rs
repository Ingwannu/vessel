//! Synthetic file persistence and permission proofs, independent of a live node.

use super::*;

fn time() -> DateTime<Utc> {
    "2026-01-01T00:00:00Z".parse().unwrap()
}

fn private_write(data: &DataDir, bytes: &[u8]) {
    files::replace(&data.providers_path(), bytes).unwrap();
}

fn copies(data: &DataDir, kind: &str) -> Vec<PathBuf> {
    fs::read_dir(data.root())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(&format!("{PROVIDERS_FILE}.{kind}-"))
        })
        .collect()
}

#[test]
fn saved_providers_roundtrip_and_are_created_owner_only() {
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path());
    assert!(data.read_provider_overrides().unwrap().is_none());
    let config: ProviderOverrides = serde_json::from_str(r#"{"local":{"extends":"acp","label":"Local","command":{"mode":"replace","program":"fake-acp","args":[]}}}"#).unwrap();
    data.save_provider_overrides(&config, time()).unwrap();
    assert_eq!(data.read_provider_overrides().unwrap(), Some(config));
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        let metadata = fs::metadata(data.providers_path()).unwrap();
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        assert_eq!(metadata.uid(), fs::metadata(root.path()).unwrap().uid());
    }
}

#[test]
fn startup_preserves_unreadable_bytes_before_selecting_manifest_defaults() {
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path());
    assert_eq!(
        data.load_provider_overrides(time()).unwrap().state,
        ProviderFileState::Missing
    );
    let broken = b"{broken synthetic file";
    private_write(&data, broken);
    let loaded = data.load_provider_overrides(time()).unwrap();
    assert_eq!(loaded.state, ProviderFileState::Recovered);
    assert_eq!(loaded.overrides, ProviderOverrides::default());
    assert!(!data.providers_path().exists());
    assert_eq!(
        fs::read(copies(&data, "unreadable").first().unwrap()).unwrap(),
        broken
    );
    data.save_provider_overrides(&loaded.overrides, time())
        .unwrap();
    assert_eq!(
        data.load_provider_overrides(time()).unwrap().state,
        ProviderFileState::Present
    );
}

#[test]
fn future_fields_at_any_depth_are_preserved_byte_for_byte_before_replacement() {
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path());
    let original =
        br#"{"later":true,"providers":{"custom":{"later_nested":{"value":"synthetic"}}}}"#;
    private_write(&data, original);
    assert!(data.read_provider_overrides().unwrap().is_some());
    data.save_provider_overrides(&ProviderOverrides::default(), time())
        .unwrap();
    let preserved = copies(&data, "later");
    assert_eq!(preserved.len(), 1);
    assert_eq!(fs::read(preserved.first().unwrap()).unwrap(), original);
    assert_eq!(
        data.read_provider_overrides().unwrap(),
        Some(ProviderOverrides::default())
    );
}

#[test]
fn malformed_files_are_set_aside_and_no_earlier_copy_is_replaced() {
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path());
    for bytes in [b"{ first".as_slice(), b"{ second".as_slice()] {
        private_write(&data, bytes);
        assert!(matches!(
            data.read_provider_overrides(),
            Err(ProvidersFileError::Decode(_))
        ));
        data.save_provider_overrides(&ProviderOverrides::default(), time())
            .unwrap();
    }
    let preserved = copies(&data, "unreadable");
    assert_eq!(preserved.len(), 2);
    let kept = preserved
        .iter()
        .map(|path| fs::read(path).unwrap())
        .collect::<Vec<_>>();
    assert!(kept.contains(&b"{ first".to_vec()));
    assert!(kept.contains(&b"{ second".to_vec()));
}

#[test]
fn a_failed_preservation_refuses_replacement_and_keeps_the_original_bytes() {
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path());
    let original = br#"{"future_option":true}"#;
    private_write(&data, original);
    let stamp = time().format("%Y%m%dT%H%M%S%.3fZ");
    for attempt in 1..=settings_file::SET_ASIDE_TRIES {
        let suffix = if attempt == 1 {
            String::new()
        } else {
            format!("-{attempt}")
        };
        fs::create_dir(
            root.path()
                .join(format!("{PROVIDERS_FILE}.later-{stamp}{suffix}")),
        )
        .unwrap();
    }
    assert!(
        data.save_provider_overrides(&ProviderOverrides::default(), time())
            .is_err()
    );
    assert_eq!(fs::read(data.providers_path()).unwrap(), original);
}

#[test]
fn an_oversized_file_is_bounded_before_json_decoding_and_preserved_on_save() {
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path());
    private_write(&data, b"{}");
    fs::OpenOptions::new()
        .write(true)
        .open(data.providers_path())
        .unwrap()
        .set_len(MAX_PROVIDER_FILE_BYTES + 1)
        .unwrap();
    assert!(matches!(
        data.read_provider_overrides(),
        Err(ProvidersFileError::TooLarge)
    ));
    data.save_provider_overrides(&ProviderOverrides::default(), time())
        .unwrap();
    let preserved = copies(&data, "unreadable");
    assert_eq!(preserved.len(), 1);
    assert_eq!(
        fs::metadata(preserved.first().unwrap()).unwrap().len(),
        MAX_PROVIDER_FILE_BYTES + 1
    );
}

#[cfg(unix)]
#[test]
fn unsafe_modes_and_symlinks_are_refused_without_mutating_the_original() {
    use std::os::unix::fs::{PermissionsExt as _, symlink};
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path());
    private_write(&data, b"{}");
    fs::set_permissions(data.providers_path(), fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(
        data.read_provider_overrides(),
        Err(ProvidersFileError::Unsafe)
    ));
    assert!(matches!(
        data.save_provider_overrides(&ProviderOverrides::default(), time()),
        Err(ProvidersFileError::Unsafe)
    ));
    assert_eq!(fs::read(data.providers_path()).unwrap(), b"{}");
    fs::remove_file(data.providers_path()).unwrap();
    let elsewhere = root.path().join("synthetic-original");
    fs::write(&elsewhere, b"{}").unwrap();
    symlink(&elsewhere, data.providers_path()).unwrap();
    assert!(matches!(
        data.save_provider_overrides(&ProviderOverrides::default(), time()),
        Err(ProvidersFileError::Unsafe)
    ));
    assert_eq!(fs::read(elsewhere).unwrap(), b"{}");
}

fn private_chain(error: &(dyn std::error::Error + 'static), marker: &str) -> String {
    let mut current = Some(error);
    let mut rendered = String::new();
    while let Some(error) = current {
        let entry = format!("{error}\n{error:?}\n");
        assert!(!entry.contains(marker));
        rendered.push_str(&entry);
        current = error.source();
    }
    rendered
}

#[test]
fn actual_file_decode_and_encode_causes_stay_private_through_the_public_chain() {
    const MARKER: &str = "provider-private-canary-d5c714";
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path());
    let bytes = format!(r#"{{"providers":{{"codex":{{"enabled":"{MARKER}"}}}}}}"#);
    let raw = serde_json::from_slice::<Document>(bytes.as_bytes()).unwrap_err();
    assert!(raw.to_string().contains(MARKER));
    private_write(&data, bytes.as_bytes());
    let error = data.read_provider_overrides().unwrap_err();
    let rendered = private_chain(&error, MARKER);
    assert!(rendered.contains("Data at line 1 column"));
    assert_eq!(fs::read(data.providers_path()).unwrap(), bytes.as_bytes());

    struct CannotEncode;
    impl Serialize for CannotEncode {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom(MARKER))
        }
    }
    let raw = serde_json::to_vec(&CannotEncode).unwrap_err();
    assert!(raw.to_string().contains(MARKER));
    let error = ProvidersFileError::Encode(raw.into());
    let _rendered = private_chain(&error, MARKER);
    let io = StoreError::io(root.path().join(MARKER))(io::Error::other(MARKER));
    let _rendered = private_chain(&ProvidersFileError::from(io), MARKER);
}
