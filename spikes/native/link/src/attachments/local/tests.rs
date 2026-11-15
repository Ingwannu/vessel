use super::*;
use vessel_app_view::composer::drafts::DraftKey;

struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        Self(std::env::temp_dir().join(format!(
            "vessel-local-bytes-{}-{serial}",
            std::process::id()
        )))
    }
    fn store(&self) -> Store {
        Store::at(&self.0).unwrap()
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        if self.0.exists() {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
fn put(store: &Store, bytes: &[u8]) -> Arc<LocalFile> {
    store
        .put_bytes(
            "sample.txt".parse().unwrap(),
            "text/plain".parse().unwrap(),
            bytes,
        )
        .unwrap()
}

#[test]
fn bytes_and_manifest_survive_reopen_and_are_private_under_the_real_test_user() {
    let root = Root::new();
    let store = root.store();
    let local = put(&store, b"snapshot");
    let manifest = Manifest::new(vec![KeptFile {
        draft: DraftKey::New,
        record: local.record.clone(),
        unconfirmed: false,
    }])
    .unwrap();
    store.save_manifest(&manifest).unwrap();
    let reopened = root.store();
    let loaded = reopened.load_manifest().unwrap();
    assert_eq!(loaded, manifest);
    let retained = reopened.get(&loaded.files()[0].record).unwrap();
    assert_eq!(std::fs::read(&retained.path).unwrap(), b"snapshot");
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        let owner = std::fs::metadata(&root.0).unwrap().uid();
        for (path, mode) in [
            (&root.0, 0o700),
            (&local.path, 0o600),
            (&root.0.join("manifest.json"), 0o600),
        ] {
            let metadata = std::fs::metadata(path).unwrap();
            assert_eq!(metadata.permissions().mode() & 0o777, mode);
            assert_eq!(metadata.uid(), owner);
        }
    }
}
#[test]
fn copied_bytes_are_immutable_and_dropping_a_lease_does_not_ignore_persisted_retention() {
    let root = Root::new();
    let store = root.store();
    let source = root.0.join("user-source.txt");
    std::fs::write(&source, b"original").unwrap();
    let local = store
        .copy_file(
            "source.txt".parse().unwrap(),
            "text/plain".parse().unwrap(),
            &source,
        )
        .unwrap();
    std::fs::write(&source, b"changed").unwrap();
    assert_eq!(std::fs::read(&local.path).unwrap(), b"original");
    let orphan = put(&store, b"orphan");
    let orphan_path = orphan.path.clone();
    let manifest = Manifest::new(vec![KeptFile {
        draft: DraftKey::New,
        record: local.record.clone(),
        unconfirmed: false,
    }])
    .unwrap();
    let future = SystemTime::now()
        .checked_add(GRACE.saturating_mul(2))
        .unwrap();
    assert_eq!(
        store.collect_at(&manifest, future).unwrap(),
        0,
        "runtime handles protect bytes"
    );
    drop(orphan);
    assert_eq!(store.collect_at(&manifest, future).unwrap(), 1);
    assert!(!orphan_path.exists());
    let path = local.path.clone();
    drop(local);
    assert_eq!(store.collect_at(&manifest, future).unwrap(), 0);
    assert!(path.exists());
    assert_eq!(store.collect_at(&Manifest::default(), future).unwrap(), 1);
    assert!(source.exists(), "unmanaged user files are never collected");
}
#[test]
fn corrupt_manifest_invalid_lengths_and_symlinks_never_become_valid_managed_bytes() {
    let root = Root::new();
    let store = root.store();
    assert!(matches!(
        store.put_bytes(
            "empty.txt".parse().unwrap(),
            "text/plain".parse().unwrap(),
            b""
        ),
        Err(LocalFailure::Invalid(LocalProblem::Size))
    ));
    std::fs::write(root.0.join("manifest.json"), b"not json").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(
            root.0.join("manifest.json"),
            std::fs::Permissions::from_mode(0o600),
        )
        .unwrap();
    }
    assert!(matches!(store.load_manifest(), Err(LocalFailure::Manifest)));
    assert_eq!(
        std::fs::read(root.0.join("manifest.json")).unwrap(),
        b"not json"
    );
    assert!(matches!(
        store.save_manifest(&Manifest::default()),
        Err(LocalFailure::Manifest)
    ));
    assert!(matches!(
        store.collect(&Manifest::default()),
        Err(LocalFailure::Manifest)
    ));
    let local = put(&store, b"fixed length");
    std::fs::write(&local.path, b"short").unwrap();
    assert!(matches!(
        store.get(&local.record),
        Err(LocalFailure::Changed)
    ));
    #[cfg(unix)]
    {
        let outside = root.0.join("outside");
        std::fs::write(&outside, b"outside").unwrap();
        std::fs::remove_file(&local.path).unwrap();
        std::os::unix::fs::symlink(&outside, &local.path).unwrap();
        assert!(matches!(
            store.get(&local.record),
            Err(LocalFailure::Boundary)
        ));
        assert_eq!(std::fs::read(outside).unwrap(), b"outside");
    }
}
#[test]
fn manifest_deserialization_enforces_duplicate_and_count_bounds() {
    let root = Root::new();
    let store = root.store();
    let local = put(&store, b"a");
    let entry = KeptFile {
        draft: DraftKey::New,
        record: local.record.clone(),
        unconfirmed: false,
    };
    assert!(Manifest::new(vec![entry.clone(), entry.clone()]).is_err());
    let invalid = serde_json::to_string(&vec![entry.clone(), entry]).unwrap();
    assert!(serde_json::from_str::<Manifest>(&invalid).is_err());
}
