//! Every file call against a temporary root, and each tries to leave it.

use vessel_proto::workspace::{EntryKind, FileContent, RelPath, WriteExpect};

use super::*;
use crate::paths::Root;
use crate::testing::Sandbox;

fn rel(text: &str) -> RelPath {
    text.parse().unwrap()
}

/// A root with a few files, a `.git` folder and a folder outside it.
fn setup() -> (tempfile::TempDir, Root, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    let outside = temp.path().join("outside");
    Sandbox::write(&root, "src/main.rs", "fn main() {}\n");
    Sandbox::write(&root, "b.txt", "b");
    Sandbox::write(&root, ".git/config", "x");
    Sandbox::write(&outside, "secret", "s");
    let opened = Root::open(&root).unwrap();
    (temp, opened, outside)
}

#[tokio::test]
async fn a_listing_puts_folders_first_and_leaves_git_out() {
    let (_temp, root, _) = setup();
    let listing = list(&root, &RelPath::root()).await.unwrap();
    let names: Vec<_> = listing
        .entries
        .iter()
        .map(|entry| (entry.name.as_str(), entry.kind))
        .collect();
    assert_eq!(names, [("src", EntryKind::Dir), ("b.txt", EntryKind::File)]);
    assert_eq!(listing.entries[1].size, Some(1));
    assert!(matches!(
        list(&root, &rel("b.txt")).await,
        Err(FileError::Kind(_, "folder"))
    ));
}

#[tokio::test]
async fn reads_are_pages_and_binary_files_are_not_sent() {
    let (_temp, root, _) = setup();
    Sandbox::write(root.path(), "long.txt", &"가".repeat(10));
    let first = read(&root, &rel("long.txt"), 0, Some(4)).await.unwrap();
    let FileContent::Text { text, next } = first.content else {
        panic!("text");
    };
    assert_eq!((text.as_str(), next), ("가", Some(3)));
    let rest = read(&root, &rel("long.txt"), 3, None).await.unwrap();
    let FileContent::Text { text, next } = rest.content else {
        panic!("text");
    };
    assert_eq!((text.chars().count(), next), (9, None));
    assert_eq!(first.version.size, 30);
    assert!(first.version.hash.is_some());
    fs::write(root.path().join("blob.bin"), b"\x7fELF\0\0").unwrap();
    assert_eq!(
        read(&root, &rel("blob.bin"), 0, None)
            .await
            .unwrap()
            .content,
        FileContent::Binary
    );
    assert!(matches!(
        read(&root, &rel("src"), 0, None).await,
        Err(FileError::Kind(_, "file"))
    ));
}

#[tokio::test]
async fn a_write_holds_to_what_it_expects() {
    let (_temp, root, _) = setup();
    let created = write(
        &root,
        &rel("new.txt"),
        "one".to_owned(),
        WriteExpect::Absent,
    )
    .await
    .unwrap();
    assert!(matches!(
        write(&root, &rel("new.txt"), "x".to_owned(), WriteExpect::Absent).await,
        Err(FileError::Exists(_))
    ));
    let guard = |version: &FileVersion| WriteExpect::Content {
        hash: version.hash.clone().unwrap(),
    };
    let replaced = write(&root, &rel("new.txt"), "two".to_owned(), guard(&created))
        .await
        .unwrap();
    // The same size, maybe the same tick of a coarse clock: the hash still
    // tells the versions apart.
    assert!(matches!(
        write(&root, &rel("new.txt"), "six".to_owned(), guard(&created)).await,
        Err(FileError::Changed(_))
    ));
    assert_eq!(
        fs::read_to_string(root.path().join("new.txt")).unwrap(),
        "two"
    );
    assert_eq!(replaced.size, 3);
    drop(
        write(&root, &rel("new.txt"), "any".to_owned(), WriteExpect::Any)
            .await
            .unwrap(),
    );
    assert!(matches!(
        write(
            &root,
            &rel("big.txt"),
            "x".repeat(MAX_WRITE + 1),
            WriteExpect::Any
        )
        .await,
        Err(FileError::TooLarge)
    ));
    assert!(matches!(
        write(
            &root,
            &rel("missing/x.txt"),
            String::new(),
            WriteExpect::Any
        )
        .await,
        Err(FileError::Path(PathError::NotFound(_)))
    ));
    // Nothing temporary is left beside it.
    let left: Vec<_> = fs::read_dir(root.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains("vessel-"))
        .collect();
    assert!(left.is_empty(), "{left:?}");
}

/// The guard is the content's hash (protocol 11), which a page holds
/// exactly: a touch that keeps the content passes, nothing there or a file
/// grown past the hashed bound is `changed`, and `modified` is
/// milliseconds, which a JavaScript number holds exactly.
#[tokio::test]
async fn a_write_is_guarded_by_the_content_it_read() {
    let (_temp, root, _) = setup();
    let first = write(
        &root,
        &rel("note.txt"),
        "one".to_owned(),
        WriteExpect::Absent,
    )
    .await
    .unwrap();
    let hash = first.hash.clone().unwrap();
    assert!(first.modified < 1 << 53, "{}", first.modified);
    // Touched, not changed: the content is what was read, so it replaces.
    let file = File::options()
        .write(true)
        .open(root.path().join("note.txt"))
        .unwrap();
    file.set_modified(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1))
        .unwrap();
    drop(file);
    let second = write(
        &root,
        &rel("note.txt"),
        "two".to_owned(),
        WriteExpect::Content { hash: hash.clone() },
    )
    .await
    .unwrap();
    assert_ne!(second.hash, first.hash);
    // Nothing there is not the content read.
    fs::remove_file(root.path().join("note.txt")).unwrap();
    assert!(matches!(
        write(
            &root,
            &rel("note.txt"),
            "three".to_owned(),
            WriteExpect::Content { hash: hash.clone() }
        )
        .await,
        Err(FileError::Changed(_))
    ));
    assert!(!root.path().join("note.txt").exists());
    // Grown past the bound, a file has no hash, so it is not the content
    // read; it is left as it is.
    let big = vec![b'a'; usize::try_from(MAX_HASHED).unwrap() + 1];
    fs::write(root.path().join("note.txt"), &big).unwrap();
    assert!(matches!(
        write(
            &root,
            &rel("note.txt"),
            "four".to_owned(),
            WriteExpect::Content { hash }
        )
        .await,
        Err(FileError::Changed(_))
    ));
    assert_eq!(
        fs::metadata(root.path().join("note.txt")).unwrap().len(),
        MAX_HASHED + 1
    );
}

#[cfg(unix)]
#[tokio::test]
async fn writes_keep_permissions_and_never_leave_through_a_link() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let (_temp, root, outside) = setup();
    let script = root.path().join("run.sh");
    fs::write(&script, "#!/bin/sh\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o750)).unwrap();
    drop(
        write(
            &root,
            &rel("run.sh"),
            "#!/bin/sh\necho\n".to_owned(),
            WriteExpect::Any,
        )
        .await
        .unwrap(),
    );
    assert_eq!(
        fs::metadata(&script).unwrap().permissions().mode() & 0o777,
        0o750
    );

    symlink(&outside, root.path().join("out")).unwrap();
    symlink(outside.join("secret"), root.path().join("secret")).unwrap();
    symlink(root.path().join("b.txt"), root.path().join("alias")).unwrap();
    for escape in ["out/new.txt", "secret"] {
        assert!(
            write(&root, &rel(escape), "x".to_owned(), WriteExpect::Any)
                .await
                .is_err(),
            "{escape}"
        );
        assert!(
            read(&root, &rel(escape), 0, None).await.is_err(),
            "{escape}"
        );
    }
    assert!(create_dir(&root, &rel("out/dir")).await.is_err());
    assert!(
        rename(&root, &rel("b.txt"), &rel("out/b.txt"))
            .await
            .is_err()
    );
    assert!(delete(&root, &rel("out/secret"), false).await.is_err());
    assert_eq!(fs::read_to_string(outside.join("secret")).unwrap(), "s");

    // Through a link inside, the file it names is written; the link stays.
    drop(
        write(
            &root,
            &rel("alias"),
            "via alias".to_owned(),
            WriteExpect::Any,
        )
        .await
        .unwrap(),
    );
    assert_eq!(
        fs::read_to_string(root.path().join("b.txt")).unwrap(),
        "via alias"
    );
    assert!(
        fs::symlink_metadata(root.path().join("alias"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    // Deleting a link removes the link, not what it names.
    delete(&root, &rel("secret"), false).await.unwrap();
    assert!(outside.join("secret").exists());
}

#[tokio::test]
async fn create_rename_and_delete() {
    let (_temp, root, _) = setup();
    create_dir(&root, &rel("docs")).await.unwrap();
    assert!(matches!(
        create_dir(&root, &rel("docs")).await,
        Err(FileError::Exists(_))
    ));
    rename(&root, &rel("b.txt"), &rel("docs/b.txt"))
        .await
        .unwrap();
    assert!(root.path().join("docs/b.txt").exists());
    Sandbox::write(root.path(), "c.txt", "c");
    assert!(matches!(
        rename(&root, &rel("c.txt"), &rel("docs/b.txt")).await,
        Err(FileError::Exists(_))
    ));
    assert!(
        rename(&root, &rel("docs"), &rel("docs/inner"))
            .await
            .is_err()
    );
    assert!(matches!(
        delete(&root, &rel("docs"), false).await,
        Err(FileError::NotEmpty(_))
    ));
    delete(&root, &rel("docs"), true).await.unwrap();
    assert!(!root.path().join("docs").exists());
    assert!(matches!(
        delete(&root, &RelPath::root(), true).await,
        Err(FileError::Path(PathError::Root))
    ));
    assert!(matches!(
        delete(&root, &rel("nothing"), false).await,
        Err(FileError::Path(PathError::NotFound(_)))
    ));
}

#[tokio::test]
async fn a_listing_marks_and_hides_what_git_ignores() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("r");
    let git = sandbox.git().await;
    Sandbox::write(&repo, ".gitignore", "target/\n");
    Sandbox::write(&repo, "target/out", "o");
    let root = Root::open(&repo).unwrap();
    let hidden = list_marked(&git, &root, &RelPath::root(), false)
        .await
        .unwrap();
    assert!(hidden.entries.iter().all(|entry| entry.name != "target"));
    let shown = list_marked(&git, &root, &RelPath::root(), true)
        .await
        .unwrap();
    let target = shown
        .entries
        .iter()
        .find(|entry| entry.name == "target")
        .unwrap();
    assert!(target.ignored);
}
