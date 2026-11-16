use super::*;
use nix::{sys::stat::Mode, unistd::mkfifo};
use std::{
    fs::{DirBuilder, File, FileTimes, OpenOptions},
    io::{Seek as _, SeekFrom, Write as _},
    os::unix::{
        ffi::OsStringExt as _,
        fs::{DirBuilderExt as _, OpenOptionsExt as _, symlink},
    },
};

fn write(root: &Path, name: &str, bytes: &[u8], mode: u32) -> File {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(mode)
        .open(root.join(name))
        .expect("owned fixture file");
    file.write_all(bytes).expect("owned fixture bytes");
    file
}
fn directory_at(path: &Path) {
    DirBuilder::new()
        .mode(0o700)
        .create(path)
        .expect("private child directory");
}
fn observe(path: &Path) -> Result<Observed, Failure> {
    Pending::begin(path, &CancellationToken::new())?.finish()
}

#[test]
fn two_actual_passes_are_order_independent_and_cover_internal_link_targets() {
    let first = tempfile::tempdir().expect("first owned candidate");
    let second = tempfile::tempdir().expect("second owned candidate");
    let metadata = fs::metadata(first.path()).expect("actual root metadata");
    assert_eq!(metadata.uid(), nix::unistd::geteuid().as_raw());
    assert_eq!(metadata.mode() & 0o077, 0);
    for (root, reverse) in [(first.path(), false), (second.path(), true)] {
        directory_at(&root.join("assets"));
        if reverse {
            let _file = write(root, "assets/data", b"data", 0o600);
            let _helper = write(root, "helper", b"helper", 0o700);
        } else {
            let _helper = write(root, "helper", b"helper", 0o700);
            let _file = write(root, "assets/data", b"data", 0o600);
        }
        symlink("assets/data", root.join("link")).expect("internal leaf link");
        symlink("assets", root.join("directory-link")).expect("no directory-link traversal");
    }
    let a = observe(first.path()).expect("complete first candidate");
    let b = observe(second.path()).expect("same facts in different creation order");
    assert_eq!(a, b);
    assert_eq!(
        a.digest.budget.entries, 5,
        "link alias adds no traversed copies"
    );
    assert_eq!(a.digest.budget.file_bytes, 10);
}

#[test]
fn same_size_restored_mtime_edit_between_full_passes_never_yields_identity() {
    let folder = tempfile::tempdir().expect("owned candidate");
    let mut file = write(folder.path(), "helper", b"first", 0o700);
    let times = file.metadata().expect("initial descriptor");
    let before = observe(folder.path()).expect("old complete fact");
    let pending =
        Pending::begin(folder.path(), &CancellationToken::new()).expect("first pass only");
    assert_eq!(file.seek(SeekFrom::Start(0)).expect("same-size rewrite"), 0);
    file.write_all(b"other").expect("new owned bytes");
    file.set_times(FileTimes::new().set_modified(times.modified().expect("original mtime")))
        .expect("restore mtime without changing ownership/mode");
    assert_eq!(file.metadata().expect("new size").len(), times.len());
    assert!(matches!(pending.finish(), Err(Failure::Changed)));
    let after = observe(folder.path()).expect("new complete fact");
    assert_ne!(before, after, "complete content is hashed on both passes");
}

#[test]
fn escape_dangling_cycle_and_fifo_never_become_complete_candidates() {
    let folder = tempfile::tempdir().expect("owned candidate");
    let outside = tempfile::tempdir().expect("outside fixture");
    let _outside = write(outside.path(), "data", b"outside", 0o600);
    let link = folder.path().join("link");
    symlink(outside.path().join("data"), &link).expect("escaping fixture");
    assert!(matches!(observe(folder.path()), Err(Failure::Policy)));
    fs::remove_file(&link).expect("remove owned fixture link");
    symlink("missing", &link).expect("dangling fixture");
    assert!(matches!(observe(folder.path()), Err(Failure::File(_))));
    fs::remove_file(&link).expect("remove dangling fixture");
    symlink("link", &link).expect("cyclic fixture");
    assert!(matches!(observe(folder.path()), Err(Failure::File(_))));
    fs::remove_file(&link).expect("remove cyclic fixture");
    mkfifo(&link, Mode::S_IRUSR | Mode::S_IWUSR).expect("owned FIFO");
    assert!(matches!(observe(folder.path()), Err(Failure::Policy)));
}

#[test]
fn aggregate_sparse_files_and_real_depth_are_refused_before_hashing() {
    let folder = tempfile::tempdir().expect("owned candidate");
    for name in ["one", "two", "three"] {
        let file = write(folder.path(), name, b"", 0o600);
        file.set_len(super::super::MAX_FILE_BYTES)
            .expect("sparse logical bytes");
    }
    assert!(matches!(
        observe(folder.path()),
        Err(Failure::Shape(Problem::Capacity(
            super::super::Limit::TotalBytes
        )))
    ));
    let deep = tempfile::tempdir().expect("owned deep candidate");
    let mut path = deep.path().to_path_buf();
    for _ in 0..=super::super::MAX_DEPTH {
        path.push("child");
        directory_at(&path);
    }
    assert!(matches!(
        observe(deep.path()),
        Err(Failure::Shape(Problem::Capacity(
            super::super::Limit::Depth
        )))
    ));
}

#[test]
fn root_link_cancel_and_shared_deadline_refuse_and_all_diagnostics_are_sealed() {
    let folder = tempfile::tempdir().expect("owned candidate");
    let _file = write(folder.path(), "helper", b"helper", 0o700);
    let cancellation = CancellationToken::new();
    let pending = Pending::begin(folder.path(), &cancellation).expect("first full pass");
    cancellation.cancel();
    assert!(matches!(
        pending.finish(),
        Err(Failure::File(file::Failure::Cancelled))
    ));
    let mut pending = Pending::begin(folder.path(), &CancellationToken::new()).expect("first pass");
    pending.scope.deadline = Instant::now();
    assert!(matches!(
        pending.finish(),
        Err(Failure::File(file::Failure::Expired))
    ));
    let holder = tempfile::tempdir().expect("link holder");
    let root_link = holder.path().join("root-link");
    symlink(folder.path(), &root_link).expect("root symlink");
    assert!(matches!(observe(&root_link), Err(Failure::Policy)));
    let missing = holder.path().join("PRIVATE-TREE-CANARY");
    let error = observe(&missing).expect_err("missing candidate");
    safe(&error);
    let raw = OsString::from_vec(b"PRIVATE-TREE-CANARY-\xff".to_vec());
    let _invalid_name = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(holder.path().join(raw))
        .expect("owned non-Unicode name fixture");
    let error = observe(holder.path()).expect_err("actual unrepresentable name");
    safe(&error);
    let cause = PathError::Outside("PRIVATE-TREE-CANARY".parse().expect("valid private name"));
    safe(&Failure::Root(PrivateRoot(cause)));
}

fn safe(error: &dyn Error) {
    let mut source = Some(error);
    while let Some(error) = source {
        assert!(!format!("{error:?} {error}").contains("PRIVATE-TREE-CANARY"));
        source = error.source();
    }
}
