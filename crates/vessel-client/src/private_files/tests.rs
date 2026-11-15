//! Real file-type boundaries and bounded decoding; no transport or real user files.
use std::io::{self, Cursor};
use std::os::unix::fs::symlink;

use super::*;

#[test]
fn exact_byte_caps_accept_unicode_and_refuse_one_more_before_decode() {
    for purpose in [Purpose::Book, Purpose::Credential] {
        let limit = purpose.limit();
        let cap = usize::try_from(limit.0).unwrap();
        let bytes = vec![b'x'; cap];
        assert_eq!(
            read_limited(&mut Cursor::new(&bytes), limit)
                .unwrap()
                .as_str()
                .len(),
            cap
        );
        let over = vec![b'x'; cap.checked_add(1).unwrap()];
        assert!(matches!(
            read_limited(&mut Cursor::new(&over), limit),
            Err(Error::TooLarge { .. })
        ));
    }
    let text = "한글😀";
    assert_eq!(
        read_limited(&mut Cursor::new(text.as_bytes()), ByteLimit(10))
            .unwrap()
            .as_str(),
        text
    );
    assert!(matches!(
        read_limited(&mut Cursor::new(text.as_bytes()), ByteLimit(9)),
        Err(Error::TooLarge { .. })
    ));
}

#[test]
fn malformed_utf8_and_every_error_source_never_retain_the_bytes() {
    let canary = "synthetic-private-material";
    let mut bytes = canary.as_bytes().to_vec();
    bytes.push(0xff);
    let error = match read_limited(&mut Cursor::new(bytes), Purpose::Credential.limit()) {
        Err(error) => error,
        Ok(_) => panic!("invalid UTF-8 must fail"),
    };
    assert!(!format!("{error:?}").contains(canary));
    let mut source: Option<&dyn std::error::Error> = Some(&error);
    while let Some(error) = source {
        assert!(!error.to_string().contains(canary));
        assert!(!format!("{error:?}").contains(canary));
        source = error.source();
    }
}

#[test]
fn a_read_requests_at_most_the_one_byte_probe() {
    struct Endless {
        read: usize,
    }
    impl io::Read for Endless {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            bytes.fill(b'x');
            self.read = self.read.checked_add(bytes.len()).unwrap();
            Ok(bytes.len())
        }
    }
    let mut input = Endless { read: 0 };
    let limit = Purpose::Credential.limit();
    assert!(matches!(
        read_limited(&mut input, limit),
        Err(Error::TooLarge { .. })
    ));
    assert_eq!(
        input.read,
        usize::try_from(limit.0.checked_add(1).unwrap()).unwrap()
    );
}

#[test]
fn the_opened_directory_does_not_follow_a_replaced_path() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    let moved = temp.path().join("moved");
    let other = temp.path().join("other");
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir(&other).unwrap();
    std::fs::write(root.join("nodes.toml"), "original").unwrap();
    std::fs::write(other.join("nodes.toml"), "replacement").unwrap();
    let directory = Directory::open(&root).unwrap();
    std::fs::rename(&root, &moved).unwrap();
    symlink(&other, &root).unwrap();
    let text = directory
        .read(Name::parse("nodes.toml").unwrap(), Purpose::Book)
        .unwrap()
        .unwrap();
    assert_eq!(text.as_str(), "original");
    assert!(matches!(Directory::open(&root), Err(Error::UnsafeTarget)));
}

#[test]
fn symlink_directory_and_traversal_are_refused_without_reading() {
    let temp = tempfile::tempdir().unwrap();
    let directory = Directory::open(temp.path()).unwrap();
    let name = Name::parse("input").unwrap();
    std::fs::write(temp.path().join("secret"), "synthetic-private-material").unwrap();
    symlink("secret", temp.path().join("input")).unwrap();
    assert!(matches!(
        directory.read(name, Purpose::Credential),
        Err(Error::UnsafeTarget)
    ));
    std::fs::remove_file(temp.path().join("input")).unwrap();
    std::fs::create_dir(temp.path().join("input")).unwrap();
    assert!(matches!(
        directory.read(name, Purpose::Book),
        Err(Error::UnsafeTarget)
    ));
    for unsafe_name in [
        "",
        ".",
        "..",
        "../secret",
        "/secret",
        "a/b",
        "a/",
        "a/.",
        "a\\b",
        "a\0b",
    ] {
        assert!(matches!(Name::parse(unsafe_name), Err(Error::UnsafeName)));
    }
    assert!(
        directory
            .read(Name::parse("missing").unwrap(), Purpose::Book)
            .unwrap()
            .is_none()
    );
}

#[test]
fn fifo_is_refused_before_waiting_for_a_writer() {
    let temp = tempfile::tempdir().unwrap();
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        &temp.path().join("input"),
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )
    .unwrap();
    let root = temp.path().to_owned();
    let (sent, received) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let directory = Directory::open(&root).unwrap();
        let result = directory.read(Name::parse("input").unwrap(), Purpose::Credential);
        assert!(sent.send(result).is_ok());
    });
    // Safety net only: removing NONBLOCK must fail this test, not hang a gate.
    let result = received
        .recv_timeout(std::time::Duration::from_secs(3))
        .expect("FIFO inspection must not wait for a writer");
    assert!(matches!(result, Err(Error::UnsafeTarget)));
    worker.join().unwrap();
}

#[test]
fn metadata_size_and_read_limits_agree_for_real_files() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("input");
    let limit = Purpose::Credential.limit();
    std::fs::write(&path, vec![b'x'; usize::try_from(limit.0).unwrap()]).unwrap();
    let directory = Directory::open(temp.path()).unwrap();
    let name = Name::parse("input").unwrap();
    assert_eq!(
        directory
            .read(name, Purpose::Credential)
            .unwrap()
            .unwrap()
            .as_str()
            .len(),
        usize::try_from(limit.0).unwrap()
    );
    let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.set_len(u64::from(limit.0).checked_add(1).unwrap())
        .unwrap();
    assert!(matches!(
        directory.read(name, Purpose::Credential),
        Err(Error::TooLarge { .. })
    ));
}
