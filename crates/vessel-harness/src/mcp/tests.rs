//! The directory the file goes in is checked at every write (Unix).

use std::fs;
use std::io;
use std::os::unix::fs::{DirBuilderExt as _, PermissionsExt as _, symlink};
use std::path::{Path, PathBuf};

use super::{FileForm, McpFile};

fn form() -> FileForm {
    FileForm {
        flag: "--mcp-config=",
        contents: "{\"mcpServers\":{}}".to_owned(),
        extension: ".json",
    }
}

/// An owner-only directory `name` in `root`.
fn private(root: &Path, name: &str) -> PathBuf {
    let dir = root.join(name);
    fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
    dir
}

fn entries(dir: &Path) -> usize {
    fs::read_dir(dir).unwrap().count()
}

#[test]
fn an_owner_only_directory_takes_the_file_and_the_guard_removes_it() {
    let root = tempfile::tempdir().unwrap();
    let dir = private(root.path(), "mcp");
    let written = McpFile::new(form(), &dir).unwrap().write().unwrap();
    let mode = fs::metadata(written.path()).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
    drop(written);
    assert_eq!(entries(&dir), 0);
}

#[test]
fn a_directory_open_to_others_is_refused_and_nothing_is_left() {
    let root = tempfile::tempdir().unwrap();
    let dir = private(root.path(), "mcp");
    for mode in [0o750, 0o705, 0o701] {
        fs::set_permissions(&dir, fs::Permissions::from_mode(mode)).unwrap();
        let refused = McpFile::new(form(), &dir).unwrap().write().unwrap_err();
        assert_eq!(refused.kind(), io::ErrorKind::PermissionDenied, "{mode:o}");
        assert_eq!(entries(&dir), 0, "{mode:o}");
    }
}

#[test]
fn a_link_to_an_owner_only_directory_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let real = private(root.path(), "real");
    let link = root.path().join("mcp");
    symlink(&real, &link).unwrap();
    let refused = McpFile::new(form(), &link).unwrap().write().unwrap_err();
    assert_eq!(refused.kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(entries(&real), 0);
}

#[test]
fn a_file_where_the_directory_should_be_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let plain = root.path().join("mcp");
    fs::write(&plain, "").unwrap();
    let refused = McpFile::new(form(), &plain).unwrap().write().unwrap_err();
    assert_eq!(refused.kind(), io::ErrorKind::PermissionDenied);
}

// The owner check (a directory of another user's, owner-only, that this
// user can still create in) needs a second account or root to arrange, so
// it is not exercised here; `owner_only::confirm` compares the new file's
// owner with the directory's, the one line it rests on.
