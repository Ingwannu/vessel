use super::*;
use nix::{
    fcntl::{FcntlArg, OFlag, fcntl, open},
    sys::stat::Mode,
    unistd::mkfifo,
};
use std::{
    fs::FileTimes,
    io::{Seek as _, SeekFrom, Write as _},
    os::unix::fs::symlink,
    path::Path,
};

fn write(path: &Path, bytes: &[u8]) -> File {
    let mut file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .expect("private fixture");
    file.write_all(bytes).expect("fixture bytes");
    file
}
fn probe(path: &Path) -> Probe {
    let metadata = fs::symlink_metadata(path).expect("fixture metadata");
    Probe::prepare(path.to_path_buf(), metadata.dev()).expect("regular private fixture")
}

#[test]
fn actual_descriptor_is_nonblocking_and_same_size_restored_time_changes_hash() {
    let folder = tempfile::tempdir().expect("private folder");
    let path = folder.path().join("helper");
    let mut file = write(&path, b"first");
    let metadata = file.metadata().expect("descriptor metadata");
    assert_eq!(metadata.uid(), nix::unistd::geteuid().as_raw());
    assert_eq!(metadata.gid(), nix::unistd::getegid().as_raw());
    assert_eq!(metadata.mode() & 0o077, 0);
    let observation = probe(&path);
    let opened = observation.open().expect("actual probe descriptor");
    let flags = fcntl(&opened, FcntlArg::F_GETFL).expect("descriptor flags");
    assert!(OFlag::from_bits_retain(flags).contains(OFlag::O_NONBLOCK));
    let cancellation = CancellationToken::new();
    let first = observation.read(&cancellation).expect("first content hash");
    let held = probe(&path);
    assert_eq!(
        file.seek(SeekFrom::Start(0))
            .expect("replace from beginning"),
        0
    );
    file.write_all(b"other")
        .expect("same-size replacement bytes");
    file.set_times(FileTimes::new().set_modified(metadata.modified().expect("mtime")))
        .expect("restore original mtime");
    assert_eq!(file.metadata().expect("size").len(), metadata.len());
    assert_eq!(
        file.metadata().expect("mtime").modified().expect("time"),
        metadata.modified().expect("original time")
    );
    assert!(matches!(held.read(&cancellation), Err(Failure::Changed)));
    let second = probe(&path)
        .read(&cancellation)
        .expect("second content hash");
    assert_ne!(first, second, "content, not restored mtime, binds bytes");
}

#[test]
fn fifo_and_link_refuse_before_read_including_replacement_after_prepare() {
    let folder = tempfile::tempdir().expect("private folder");
    let path = folder.path().join("candidate");
    let file = write(&path, b"bytes");
    let held = probe(&path);
    drop(file);
    fs::remove_file(&path).expect("replace owned fixture");
    mkfifo(&path, Mode::S_IRUSR | Mode::S_IWUSR).expect("owned FIFO fixture");
    let device = fs::symlink_metadata(&path).expect("FIFO").dev();
    assert!(matches!(
        Probe::prepare(path.clone(), device),
        Err(Failure::Policy)
    ));
    // A controlled peer keeps this regression bounded even if NONBLOCK is
    // accidentally removed. The regular-file case independently checks flags.
    let _peer = open(&path, OFlag::O_RDWR | OFlag::O_NONBLOCK, Mode::empty())
        .expect("controlled FIFO peer");
    assert!(matches!(
        held.read(&CancellationToken::new()),
        Err(Failure::Policy)
    ));
    fs::remove_file(&path).expect("replace owned FIFO");
    let target = folder.path().join("target");
    let _file = write(&target, b"target");
    symlink(&target, &path).expect("owned link fixture");
    assert!(matches!(Probe::prepare(path, device), Err(Failure::Policy)));
}

#[test]
fn sparse_over_budget_device_mismatch_cancel_and_expiry_refuse() {
    let folder = tempfile::tempdir().expect("private folder");
    let path = folder.path().join("candidate");
    let file = write(&path, b"bounded");
    let device = file.metadata().expect("metadata").dev();
    assert!(matches!(
        Probe::prepare(path.clone(), device ^ 1),
        Err(Failure::Policy)
    ));
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert!(matches!(
        probe(&path).read(&cancelled),
        Err(Failure::Cancelled)
    ));
    assert!(matches!(
        probe(&path).read_until(&CancellationToken::new(), Instant::now()),
        Err(Failure::Expired)
    ));
    file.set_len(MAX_FILE_BYTES.saturating_add(1))
        .expect("sparse fixture without allocating file bytes");
    assert!(matches!(
        Probe::prepare(path, device),
        Err(Failure::Capacity(Problem::Capacity(Limit::FileBytes)))
    ));
}

#[test]
fn concrete_io_cause_is_retained_but_every_exposed_diagnostic_is_sealed() {
    let folder = tempfile::tempdir().expect("private folder");
    let missing = folder.path().join("PRIVATE-ARTIFACT-IO-CANARY");
    let Err(error) = Probe::prepare(missing, 0) else {
        panic!("missing fixture accepted");
    };
    let mut source: Option<&(dyn Error + 'static)> = Some(&error);
    while let Some(error) = source {
        assert!(!format!("{error} {error:?}").contains("PRIVATE-ARTIFACT-IO-CANARY"));
        source = error.source();
    }
    let synthetic = io_failure(io::Error::other("PRIVATE-ARTIFACT-IO-CANARY"));
    let Failure::Io(cause) = synthetic else {
        panic!("concrete cause lost");
    };
    assert!(cause.0.to_string().contains("PRIVATE-ARTIFACT-IO-CANARY"));
    assert!(!format!("{cause:?} {cause}").contains("PRIVATE-ARTIFACT-IO-CANARY"));
    assert!(cause.source().is_none());
}
