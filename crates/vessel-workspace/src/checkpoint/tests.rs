//! Real isolated Git/filesystem fixtures for the staged capture path.
#[cfg(target_os = "linux")]
mod linux {
    use super::super::{Error, archive::Archive, capture, enumeration, rooted::Root};
    use crate::testing::Sandbox;
    use std::{
        fs,
        os::unix::{ffi::OsStringExt as _, fs::PermissionsExt as _},
    };
    use vessel_proto::checkpoint::{CheckpointError, FileState};

    fn archive(sandbox: &Sandbox) -> Archive {
        let path = sandbox.path().join("archive");
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Archive::open(path).unwrap()
    }
    #[tokio::test]
    async fn dirty_tracked_nonignored_untracked_and_binary_bytes_survive_capture() {
        let sandbox = Sandbox::new();
        let dir = sandbox.repo("project");
        let git = sandbox.git().await;
        fs::write(dir.join("README.md"), b"dirty\0binary").unwrap();
        fs::write(dir.join("new"), b"untracked").unwrap();
        fs::write(dir.join(".gitignore"), "ignored\n").unwrap();
        fs::write(dir.join("ignored"), b"excluded").unwrap();
        let paths = enumeration::paths(&git, &dir).await.unwrap();
        assert!(!paths.iter().any(|path| path.to_string() == "ignored"));
        let archive = archive(&sandbox);
        let root = Root::open(&dir).unwrap();
        let captured = capture::image(&root, &archive, &paths).unwrap();
        let state = captured
            .entries()
            .iter()
            .find(|entry| entry.path.to_string() == "README.md")
            .unwrap();
        let FileState::Present { digest, bytes, .. } = state.state else {
            panic!("regular tracked entry expected")
        };
        assert_eq!(bytes.get(), u64::try_from(b"dirty\0binary".len()).unwrap());
        assert_eq!(fs::read(archive.path(digest)).unwrap(), b"dirty\0binary");
        let second = capture::image(&root, &archive, &paths).unwrap();
        assert_eq!(captured, second);
    }
    #[tokio::test]
    async fn missing_tracked_files_are_absent_not_empty_blobs() {
        let sandbox = Sandbox::new();
        let dir = sandbox.repo("project");
        let git = sandbox.git().await;
        fs::remove_file(dir.join("README.md")).unwrap();
        let paths = enumeration::paths(&git, &dir).await.unwrap();
        let image = capture::image(&Root::open(&dir).unwrap(), &archive(&sandbox), &paths).unwrap();
        assert_eq!(image.entries().first().unwrap().state, FileState::Absent);
    }
    #[tokio::test]
    async fn invalid_utf8_is_refused_instead_of_lossy_skip_or_replacement() {
        let sandbox = Sandbox::new();
        let dir = sandbox.repo("project");
        let git = sandbox.git().await;
        fs::write(dir.join(std::ffi::OsString::from_vec(vec![0xff])), b"bytes").unwrap();
        assert!(matches!(
            enumeration::paths(&git, &dir).await,
            Err(Error::Refused(CheckpointError::InvalidPath))
        ));
    }
    #[tokio::test]
    async fn symlink_hardlink_and_fifo_are_refused_before_any_file_is_read() {
        for kind in ["symlink", "hardlink", "fifo"] {
            let sandbox = Sandbox::new();
            let dir = sandbox.repo("project");
            let git = sandbox.git().await;
            let source = sandbox.path().join("outside");
            fs::write(&source, b"private").unwrap();
            let target = dir.join("unsafe");
            match kind {
                "symlink" => std::os::unix::fs::symlink(&source, &target).unwrap(),
                "hardlink" => fs::hard_link(&source, &target).unwrap(),
                "fifo" => nix::unistd::mkfifo(&target, nix::sys::stat::Mode::S_IRWXU).unwrap(),
                _ => panic!("fixture kind"),
            }
            // A FIFO isn't emitted by all Git versions: test the root guard directly.
            let path = vessel_proto::checkpoint::CheckpointPath::try_from(
                "unsafe"
                    .parse::<vessel_proto::workspace::RelPath>()
                    .unwrap(),
            )
            .unwrap();
            assert!(matches!(
                Root::open(&dir).unwrap().file(&path),
                Err(Error::Refused(CheckpointError::UnsafeEntry))
            ));
            let _paths = enumeration::paths(&git, &dir).await.unwrap();
            assert_eq!(fs::read(source).unwrap(), b"private");
        }
    }
    #[test]
    #[ignore = "requires effective-image staging and per-file restore implementation"]
    fn restore_preserves_nonroot_ownership_and_tightened_access() {}
    #[test]
    #[ignore = "requires journal-integrated fingerprint rollback implementation"]
    fn io_failure_rolls_back_only_our_writes_and_keeps_external_edits() {}
}
