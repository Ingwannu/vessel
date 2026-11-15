use super::*;

fn source(text: &str) -> ConfigText {
    ConfigText::parse(text).unwrap()
}

#[tokio::test]
async fn missing_empty_and_external_same_size_changes_have_distinct_revisions() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().to_path_buf();
    let missing = read(root.clone()).await.unwrap();
    assert_eq!(missing.revision, ConfigRevision::Missing);
    let empty = write(root.clone(), missing.revision, source(""))
        .await
        .unwrap();
    assert_ne!(empty.revision, ConfigRevision::Missing);
    assert!(matches!(
        write(root.clone(), ConfigRevision::Missing, source("# x")).await,
        Err(ConfigEditError::Write(files::FileError::Exists(_)))
    ));
    let before = write(root.clone(), empty.revision, source("# a"))
        .await
        .unwrap();
    std::fs::write(root.join(project_file::FILE_NAME), "# b").unwrap();
    assert!(matches!(
        write(root.clone(), before.revision, source("# c")).await,
        Err(ConfigEditError::Write(files::FileError::Changed(_)))
    ));
    assert_eq!(
        std::fs::read_to_string(root.join(project_file::FILE_NAME)).unwrap(),
        "# b"
    );
}

#[tokio::test]
async fn malformed_source_can_be_repaired_but_an_invalid_save_changes_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().to_path_buf();
    let path = root.join(project_file::FILE_NAME);
    std::fs::write(&path, "[worktree\nsetup = 'broken'").unwrap();
    let invalid = read(root.clone()).await.unwrap();
    assert_eq!(invalid.validity, ConfigValidity::Invalid);
    let rejected_source = "[worktree]\nsetup = 'PRIVATE_CONFIG_CONTENT_CANARY\n";
    let error = write(
        root.clone(),
        invalid.revision.clone(),
        source(rejected_source),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, ConfigEditError::Invalid(_)));
    assert!(
        std::error::Error::source(&error)
            .unwrap()
            .is::<ConfigCause>()
    );
    assert!(!format!("{error:?}").contains(rejected_source));
    assert!(!error.to_string().contains(rejected_source));
    let mut cause = std::error::Error::source(&error);
    while let Some(current) = cause {
        assert!(!format!("{current:?} {current}").contains("PRIVATE_CONFIG_CONTENT_CANARY"));
        cause = current.source();
    }
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        invalid.text.as_str()
    );
    let text = "# keep comment\n[unknown]\nfuture_key = 'retained'\n[worktree]\nsetup = ['echo prepare']\n";
    let saved = write(root.clone(), invalid.revision, source(text))
        .await
        .unwrap();
    assert_eq!(saved.validity, ConfigValidity::Valid);
    assert_eq!(std::fs::read_to_string(path).unwrap(), text);
    assert_eq!(read(root).await.unwrap(), saved);
}

#[tokio::test]
async fn reads_share_the_existing_bound_and_refuse_binary_and_missing_roots() {
    assert_eq!(
        vessel_proto::registry::MAX_CONFIG_BYTES,
        project_file::MAX_BYTES
    );
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().to_path_buf();
    let path = root.join(project_file::FILE_NAME);
    std::fs::write(&path, vec![b'#'; project_file::MAX_BYTES + 1]).unwrap();
    assert!(matches!(
        read(root.clone()).await,
        Err(ConfigEditError::Read(_))
    ));
    std::fs::write(&path, [0xff]).unwrap();
    assert!(matches!(
        read(root.clone()).await,
        Err(ConfigEditError::NotText(_))
    ));
    assert!(matches!(
        read(root.join("absent")).await,
        Err(ConfigEditError::Root(_))
    ));
}

#[cfg(unix)]
#[tokio::test]
async fn saving_preserves_existing_permissions_and_cannot_write_through_an_external_link() {
    use std::os::unix::{
        fs::{PermissionsExt as _, symlink},
        prelude::MetadataExt as _,
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    std::fs::create_dir(&root).unwrap();
    let path = root.join(project_file::FILE_NAME);
    std::fs::write(&path, "# before").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
    let original = std::fs::metadata(&path).unwrap();
    let document = read(root.clone()).await.unwrap();
    let _saved = write(root.clone(), document.revision, source("# after"))
        .await
        .unwrap();
    let now = std::fs::metadata(&path).unwrap();
    assert_eq!(now.permissions().mode(), original.permissions().mode());
    assert_eq!(now.uid(), original.uid());
    std::fs::remove_file(&path).unwrap();
    let outside = temp.path().join("outside.toml");
    std::fs::write(&outside, "# private outside").unwrap();
    symlink(&outside, &path).unwrap();
    let hidden = read(root.clone()).await.unwrap();
    assert_eq!(hidden.revision, ConfigRevision::Missing);
    assert!(!hidden.text.as_str().contains("private outside"));
    assert!(
        write(root, hidden.revision, source("# must not overwrite"))
            .await
            .is_err()
    );
    assert_eq!(
        std::fs::read_to_string(outside).unwrap(),
        "# private outside"
    );
}
