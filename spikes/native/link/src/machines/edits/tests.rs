//! Real temporary book files, not mocks of our writer. The CLI owns editing.
use super::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "vessel-machine-book-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        Self(root)
    }
    fn path(&self) -> PathBuf {
        self.0.join("nodes.toml")
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
const BOOK: &str = "# keep this comment\ndefault_node = \"laptop\"\n[nodes.laptop]\nurl = \"ws://127.0.0.1:7000/rpc\"\ntoken = { file = \"test-token\" }\n[nodes.builder]\nurl = \"ws://127.0.0.1:7001/rpc\"\n";
fn name(text: &str) -> Name {
    Name::parse(text).unwrap()
}
fn names(path: &Path) -> Vec<(String, bool)> {
    read(path, |_| None)
        .unwrap()
        .into_iter()
        .map(|m| (m.name.as_str().to_owned(), m.default))
        .collect()
}

#[test]
fn rename_preserves_credentials_comments_and_default_then_forget_removes_default() {
    let dir = Scratch::new();
    let path = dir.path();
    std::fs::write(&path, BOOK).unwrap();
    let original = BookFile::open(&path)
        .unwrap()
        .book()
        .get("laptop")
        .unwrap()
        .1
        .clone();
    write(
        &path,
        &Edit::Rename {
            from: name("laptop"),
            to: name("workstation"),
        },
    )
    .unwrap();
    let file = BookFile::open(&path).unwrap();
    assert_eq!(
        file.book().default_node.as_ref().map(NodeName::as_str),
        Some("workstation")
    );
    assert_eq!(file.book().get("workstation").unwrap().1, &original);
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .starts_with("# keep this comment\n")
    );
    assert_eq!(
        names(&path),
        vec![
            ("builder".to_owned(), false),
            ("workstation".to_owned(), true)
        ]
    );
    write(&path, &Edit::MakeDefault(name("builder"))).unwrap();
    write(&path, &Edit::Forget(name("builder"))).unwrap();
    assert_eq!(names(&path), vec![("workstation".to_owned(), false)]);
}

#[test]
fn missing_and_taken_changes_leave_the_original_bytes() {
    let dir = Scratch::new();
    let path = dir.path();
    std::fs::write(&path, BOOK).unwrap();
    for (edit, expected) in [
        (Edit::Forget(name("gone")), Failure::Missing),
        (Edit::MakeDefault(name("gone")), Failure::Missing),
        (
            Edit::Rename {
                from: name("laptop"),
                to: name("builder"),
            },
            Failure::Taken,
        ),
    ] {
        assert_eq!(write(&path, &edit).unwrap_err().failure(), expected);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), BOOK);
    }
}

#[test]
fn a_write_rereads_instead_of_overwriting_an_external_edit() {
    let dir = Scratch::new();
    let path = dir.path();
    std::fs::write(&path, BOOK).unwrap();
    assert_eq!(names(&path).len(), 2);
    let mut external = BookFile::open(&path).unwrap();
    external
        .rename("builder", &"other".parse().unwrap())
        .unwrap();
    write(&path, &Edit::MakeDefault(name("laptop"))).unwrap();
    assert_eq!(
        names(&path),
        vec![("laptop".to_owned(), true), ("other".to_owned(), false)]
    );
}

#[test]
fn malformed_books_and_missing_files_have_safe_typed_results() {
    let dir = Scratch::new();
    let path = dir.path();
    assert!(read(&path, |_| None).unwrap().is_empty());
    std::fs::write(
        &path,
        "token = 'canary-credential'\n[nodes.x]\nurl = [broken",
    )
    .unwrap();
    let error = read(&path, |_| None).unwrap_err();
    assert!(matches!(error.failure(), Failure::File(_)));
    assert_eq!(error.to_string(), "the machine book could not be read");
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert_eq!(read(&path, |_| None).unwrap_err().failure(), Failure::Read);
}

#[cfg(unix)]
#[test]
fn every_replacement_keeps_the_book_owner_only() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = Scratch::new();
    let path = dir.path();
    std::fs::write(&path, BOOK).unwrap();
    for edit in [
        Edit::MakeDefault(name("builder")),
        Edit::Rename {
            from: name("laptop"),
            to: name("desktop"),
        },
        Edit::Forget(name("builder")),
    ] {
        write(&path, &edit).unwrap();
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[cfg(unix)]
#[test]
fn a_failed_write_keeps_the_existing_book() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = Scratch::new();
    let path = dir.path();
    std::fs::write(&path, BOOK).unwrap();
    std::fs::set_permissions(&dir.0, std::fs::Permissions::from_mode(0o500)).unwrap();
    let result = write(&path, &Edit::MakeDefault(name("builder")));
    // Restore before assertions so failure cannot leave an unremovable fixture.
    std::fs::set_permissions(&dir.0, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(result.unwrap_err().failure(), Failure::Write);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), BOOK);
}
