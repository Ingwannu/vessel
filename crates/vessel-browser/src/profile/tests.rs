//! Actual service-identity filesystem probes; no browser or credential fixture.
use super::*;
use std::os::unix::fs::{PermissionsExt as _, symlink};
#[test]
fn creation_is_private_repeatable_and_the_lease_excludes_another_owner() {
    let parent = tempfile::tempdir_in("/tmp").unwrap();
    let root = parent.path().join("profile");
    let temporary = parent.path().join("tmp");
    let profile = Profile::prepare(&root, &temporary).unwrap();
    assert_eq!(profile.root(), root);
    assert_eq!(profile.temporary(), temporary);
    for path in [
        profile.root().to_owned(),
        profile.home(),
        profile.config(),
        profile.cache(),
        profile.cache().join("fontconfig"),
        profile.temporary().to_owned(),
    ] {
        assert_eq!(
            fs::metadata(path).unwrap().mode() & 0o777,
            PRIVATE_DIRECTORY_MODE
        );
    }
    assert!(matches!(
        Profile::prepare(&root, &temporary),
        Err(ProfileError::Busy)
    ));
    assert!(!format!("{profile:?}").contains(parent.path().to_str().unwrap()));
    drop(profile);
    assert!(Profile::prepare(&root, &temporary).is_ok());
}
#[test]
fn unsafe_existing_cache_or_link_is_refused_without_modifying_its_mode() {
    let parent = tempfile::tempdir_in("/tmp").unwrap();
    let root = parent.path().join("profile");
    let tmp = parent.path().join("tmp");
    let profile = Profile::prepare(&root, &tmp).unwrap();
    let cache = profile.cache().join("fontconfig");
    drop(profile);
    fs::set_permissions(&cache, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(
        Profile::prepare(&root, &tmp),
        Err(ProfileError::Permissions)
    ));
    assert_eq!(fs::metadata(&cache).unwrap().mode() & 0o777, 0o755);
    fs::set_permissions(&cache, fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE)).unwrap();
    fs::remove_file(root.join(".vessel-browser.lock")).unwrap();
    let canary = parent.path().join("canary");
    fs::write(&canary, b"unchanged").unwrap();
    symlink(&canary, root.join(".vessel-browser.lock")).unwrap();
    assert!(Profile::prepare(&root, &tmp).is_err());
    assert_eq!(fs::read(&canary).unwrap(), b"unchanged");
}
#[test]
fn writable_ancestors_and_long_runtime_temp_paths_fail_before_launch() {
    let parent = tempfile::tempdir_in("/tmp").unwrap();
    let tmp = parent.path().join("tmp");
    let long = parent.path().join("x".repeat(MAX_RUNTIME_TEMP_PATH_BYTES));
    assert!(matches!(
        Profile::prepare(&parent.path().join("profile"), &long),
        Err(ProfileError::TemporaryPath)
    ));
    let writable = parent.path().join("writable");
    fs::create_dir(&writable).unwrap();
    fs::set_permissions(&writable, fs::Permissions::from_mode(0o777)).unwrap();
    assert!(matches!(
        Profile::prepare(&writable.join("profile"), &tmp),
        Err(ProfileError::Permissions)
    ));
    assert!(!writable.join("profile").exists());
}
