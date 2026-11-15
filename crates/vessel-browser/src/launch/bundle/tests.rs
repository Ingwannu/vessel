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

/// Explicit export-host fixture only; the build VM has no approved CFT provision.
/// This reads/observes the fixed bundle and starts no browser or supported engine.
#[tokio::test]
#[ignore = "requires the approved root-owned CFT bundle and non-root service account"]
async fn approved_candidate_bundle_is_readable_as_service_account() {
    assert!(!nix::unistd::geteuid().is_root());
    let scope =
        crate::preparation::Preparation::for_profile(tokio_util::sync::CancellationToken::new());
    let candidate = scope
        .run(super::super::identity::Candidate::inspect)
        .await
        .unwrap()
        .unwrap();
    let fingerprint = scope
        .run(move || candidate.verify_bundle())
        .await
        .unwrap()
        .unwrap();
    assert_ne!(fingerprint.0, [0; 32]);
    scope.finish().await;
}
