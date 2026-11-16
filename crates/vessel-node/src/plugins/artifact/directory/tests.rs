use super::*;
use crate::plugins::artifact::package_path;
use nix::{
    fcntl::{FcntlArg, FdFlag, fcntl},
    sys::stat::Mode,
    unistd::mkfifo,
};
use std::{
    cell::Cell,
    fs::{self, DirBuilder},
    io::{Read as _, Write as _},
    os::unix::fs::{DirBuilderExt as _, PermissionsExt as _, symlink},
};

fn private_root() -> tempfile::TempDir {
    tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .expect("initially private disposable directory")
}

fn directory(path: &Path) {
    DirBuilder::new()
        .mode(0o700)
        .create(path)
        .expect("owned directory");
}

fn write(path: &Path, bytes: &[u8]) {
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .expect("owned file")
        .write_all(bytes)
        .expect("owned bytes");
}

#[test]
fn retained_descriptor_does_not_reopen_the_replaced_root_path() {
    let parent = private_root();
    let selected = parent.path().join("selected");
    directory(&selected);
    directory(&selected.join("assets"));
    write(&selected.join("assets/value"), b"original");
    let probe = DirectoryProbe::open(&selected).expect("actual checked descriptor");
    fs::rename(&selected, parent.path().join("retained")).expect("move original entry");
    directory(&selected);
    directory(&selected.join("assets"));
    write(&selected.join("assets/value"), b"replacement");
    let assets = probe
        .open_directory(&package_path("assets").expect("package directory"))
        .expect("retained original directory");
    let mut file = assets
        .open_file(&package_path("value").expect("package entry"))
        .expect("retained original file");
    let mut bytes = String::new();
    let _read = file
        .read_to_string(&mut bytes)
        .expect("owned bounded fixture bytes");
    assert_eq!(bytes, "original");
    assert_eq!(
        fs::read(selected.join("assets/value")).expect("other tree"),
        b"replacement"
    );
}

#[test]
fn special_files_and_symlink_ancestors_refuse_without_content_open() {
    let root = private_root();
    directory(&root.path().join("real"));
    directory(&root.path().join("real/child"));
    write(&root.path().join("real/value"), b"value");
    symlink("real", root.path().join("alias")).expect("internal directory link");
    symlink("real/value", root.path().join("link")).expect("leaf link");
    mkfifo(&root.path().join("fifo"), Mode::S_IRUSR | Mode::S_IWUSR).expect("owned fifo");
    let probe = DirectoryProbe::open(root.path()).expect("checked descriptor");
    assert!(
        probe
            .open_directory(&package_path("alias/child").expect("path"))
            .is_err()
    );
    for name in ["fifo", "link"] {
        let opened = Cell::new(false);
        let result =
            probe.open_file_with_gate(&package_path(name).expect("path"), || opened.set(true));
        assert!(matches!(result, Err(Failure::File(file::Failure::Policy))));
        assert!(
            !opened.get(),
            "special kind must refuse before the open gate"
        );
    }
}

#[test]
fn actual_file_flags_and_replaced_inode_are_checked() {
    let root = private_root();
    let value = root.path().join("value");
    write(&value, b"first");
    let probe = DirectoryProbe::open(root.path()).expect("checked descriptor");
    let path = package_path("value").expect("package path");
    let opened = probe.open_file(&path).expect("actual regular descriptor");
    let flags = OFlag::from_bits_truncate(fcntl(&opened, FcntlArg::F_GETFL).expect("actual flags"));
    assert!(flags.contains(OFlag::O_NONBLOCK));
    let descriptor_flags =
        FdFlag::from_bits_truncate(fcntl(&opened, FcntlArg::F_GETFD).expect("descriptor flags"));
    assert!(descriptor_flags.contains(FdFlag::FD_CLOEXEC));
    assert!(matches!(
        probe.open_file_with_gate(&path, || {
            fs::rename(&value, root.path().join("held")).expect("retain old inode");
            write(&value, b"second");
        }),
        Err(Failure::File(file::Failure::Changed))
    ));
}

#[test]
fn actual_kernel_mount_boundary_refuses_before_directory_policy() {
    // This intentionally unqualified root descriptor is test-only: it exercises
    // the real kernel mount boundary, not private-root/bootstrap admission.
    let root = File::open("/").expect("read-only filesystem root");
    let device = root.metadata().expect("actual root metadata").dev();
    assert_ne!(
        device,
        fs::metadata("/proc").expect("actual proc mount").dev()
    );
    let probe = DirectoryProbe { root, device };
    assert!(matches!(
        probe.open_directory(&package_path("proc").expect("package spelling")),
        Err(Failure::System(Errno::EXDEV))
    ));
}

#[test]
fn unqualified_root_policy_and_all_error_chains_stay_explicit() {
    let shared = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o755))
        .tempdir()
        .expect("initially nonprivate fixture");
    assert_ne!(
        fs::metadata(shared.path())
            .expect("actual fixture mode")
            .mode()
            & 0o077,
        0,
        "runner umask must permit this negative fixture; no permission repair"
    );
    assert!(matches!(
        DirectoryProbe::open(shared.path()),
        Err(Failure::Directory(tree::Failure::Policy))
    ));
    let root = private_root();
    let probe = DirectoryProbe::open(root.path()).expect("private descriptor");
    let error = probe
        .open_directory(&package_path("PRIVATE-CANARY").expect("path"))
        .err()
        .expect("missing entry refusal");
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(&error);
    while let Some(cause) = source {
        assert!(!format!("{cause:?} {cause}").contains("PRIVATE-CANARY"));
        source = cause.source();
    }
    for cause in [Errno::ENOSYS, Errno::EINVAL, Errno::E2BIG] {
        assert!(matches!(system(cause), Failure::Unsupported(actual) if actual == cause));
    }
}
