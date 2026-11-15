//! Hash/real-filesystem refusal fixtures only. No positive runtime certificate.
use super::*;
fn fingerprint(name: &str, mode: u32, length: u64, digest: [u8; 32]) -> Fingerprint {
    let mut hash = ManifestHash::new();
    hash.file(Path::new(name), mode, length, digest).unwrap();
    hash.finish()
}
#[test]
fn path_permission_length_and_content_changes_are_distinct_observations() {
    let baseline = fingerprint("resources.pak", 0o644, 3, [1; 32]);
    assert_eq!(baseline, fingerprint("resources.pak", 0o644, 3, [1; 32]));
    for changed in [
        fingerprint("other.pak", 0o644, 3, [1; 32]),
        fingerprint("resources.pak", 0o600, 3, [1; 32]),
        fingerprint("resources.pak", 0o644, 4, [1; 32]),
        fingerprint("resources.pak", 0o644, 3, [2; 32]),
    ] {
        assert_ne!(baseline, changed);
    }
    let mut directory = ManifestHash::new();
    directory
        .directory(Path::new("resources.pak"), 0o644)
        .unwrap();
    assert_ne!(baseline, directory.finish());
}
#[test]
fn a_writable_service_owned_directory_is_not_a_runtime_bundle() {
    let fixture = tempfile::tempdir_in("/tmp").unwrap();
    assert!(matches!(
        Bundle::inspect(fixture.path()),
        Err(LaunchError::Permissions)
    ));
}

#[test]
fn aggregate_hash_budget_counts_read_bytes_instead_of_only_initial_metadata() {
    let mut remaining = 3;
    let first =
        super::super::identity::digest_with_budget(&mut &b"ab"[..], &mut remaining).unwrap();
    assert_eq!(remaining, 1);
    assert!(matches!(
        super::super::identity::digest_with_budget(&mut &b"cd"[..], &mut remaining),
        Err(LaunchError::Identity)
    ));
    assert_ne!(first, [0; 32]);
}
