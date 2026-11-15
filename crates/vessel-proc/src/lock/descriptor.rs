//! Relate a kernel row to a candidate's actual locked file description.
//! Generation and namespace checks bracket capture; no signal is authorized.

use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::os::fd::AsRawFd as _;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::kernel::{descriptor_writer, identity};
use super::record::Record;
use super::{KernelWriter, NodeProcess};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Descriptor(u32);

impl Descriptor {
    fn parse(name: &OsStr) -> Result<Self, Error> {
        let text = name.to_str().ok_or(Error::NameEncoding)?;
        if !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(Error::NameSyntax);
        }
        text.parse().map(Self).map_err(Error::Number)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct DirectoryIdentity {
    device: u64,
    inode: u64,
}

#[derive(Debug)]
struct Directory {
    file: File,
    identity: DirectoryIdentity,
}

impl PartialEq for Directory {
    fn eq(&self, other: &Self) -> bool {
        self.identity == other.identity
    }
}
impl Eq for Directory {}

impl Directory {
    fn open(root: &Path, process: NodeProcess) -> io::Result<Self> {
        let file = File::open(root.join(process.0.to_string()))
            .map_err(Error::Process)
            .map_err(io::Error::other)?;
        let metadata = file.metadata()?;
        if !metadata.is_dir() {
            return Err(io::Error::other(Error::NotDirectory));
        }
        Ok(Self {
            file,
            identity: DirectoryIdentity {
                device: metadata.dev(),
                inode: metadata.ino(),
            },
        })
    }

    fn path(&self) -> PathBuf {
        // The retained proc directory does not reserve a PID. Its children
        // cannot silently retarget another task after that PID is reused.
        Path::new("/proc/self/fd").join(self.file.as_raw_fd().to_string())
    }
}

/// Retain the directory used for the observation, never the selected lock itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Proof {
    directory: Arc<Directory>,
    descriptor: Descriptor,
    generation: super::generation::Proof,
}

#[derive(Debug)]
struct Possession {
    directory: Arc<Directory>,
    descriptor: Descriptor,
}

struct DescriptorScan {
    directory: Arc<Directory>,
}

impl DescriptorScan {
    const MAX_ENTRIES: usize = 4_096;

    fn open(root: &Path, process: NodeProcess) -> io::Result<Self> {
        Ok(Self {
            directory: Arc::new(Directory::open(root, process)?),
        })
    }

    fn find(self, selected: &KernelWriter) -> io::Result<Possession> {
        let mut count = 0usize;
        for entry in std::fs::read_dir(self.directory.path().join("fd"))? {
            let entry = entry?;
            // The cap is reached before the counter's representable ceiling.
            count = count.saturating_add(1);
            if count > Self::MAX_ENTRIES {
                return Err(io::Error::other(Error::TooMany));
            }
            let descriptor = Descriptor::parse(&entry.file_name()).map_err(io::Error::other)?;
            let observed = match identity(&entry.path()) {
                Ok(file) => file,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            };
            if observed != selected.file {
                continue;
            }
            // Other files' potentially large fdinfo records are outside this
            // query; only a matching file's lock lines can establish possession.
            let info = Record::read(
                &self
                    .directory
                    .path()
                    .join("fdinfo")
                    .join(descriptor.0.to_string()),
            )?;
            if matches_selected(&info, selected)? {
                return Ok(Possession {
                    directory: self.directory,
                    descriptor,
                });
            }
        }
        Err(io::Error::other(Error::Missing))
    }
}

impl Proof {
    /// Capture descriptor possession from the real proc filesystem only.
    ///
    /// # Errors
    /// The bounded query cannot prove possession; no record or lock is changed.
    pub(super) fn capture(selected: &KernelWriter) -> io::Result<Self> {
        let scan = DescriptorScan::open(Path::new("/proc"), selected.process)?;
        let generation =
            super::generation::Proof::capture(&scan.directory.path(), selected.process)?;
        let possession = scan.find(selected)?;
        generation.confirm(&possession.directory.path())?;
        Ok(Self {
            directory: possession.directory,
            descriptor: possession.descriptor,
            generation,
        })
    }
}

fn matches_selected(record: &Record, selected: &KernelWriter) -> io::Result<bool> {
    let mut found = false;
    for line in record.bytes().split(|byte| *byte == b'\n') {
        // pos/flags and filesystem-specific fdinfo are opaque here. Decode
        // only the kernel's lock block, with the existing selected-file parser.
        let Some(line) = line.strip_prefix(b"lock:") else {
            continue;
        };
        let text = std::str::from_utf8(line)
            .map_err(Error::Encoding)
            .map_err(io::Error::other)?;
        let Some(lock) = descriptor_writer(text, &selected.file)? else {
            continue;
        };
        if lock.process != selected.process {
            return Err(io::Error::other(Error::DifferentProcess));
        }
        if found {
            return Err(io::Error::other(Error::Multiple));
        }
        found = true;
    }
    Ok(found)
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("cannot open the candidate process directory")]
    Process(#[source] io::Error),
    #[error("the candidate process reference is not a directory")]
    NotDirectory,
    #[error("descriptor name is not Unicode")]
    NameEncoding,
    #[error("descriptor name is not a decimal number")]
    NameSyntax,
    #[error("descriptor number cannot be decoded")]
    Number(#[source] std::num::ParseIntError),
    #[error("descriptor lock record is not valid Unicode")]
    Encoding(#[source] std::str::Utf8Error),
    #[error(
        "candidate descriptor scan exceeds {} entries",
        DescriptorScan::MAX_ENTRIES
    )]
    TooMany,
    #[error("candidate has no descriptor owning the selected lock")]
    Missing,
    #[error("descriptor lock names a different process")]
    DifferentProcess,
    #[error("descriptor has more than one selected lock writer")]
    Multiple,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use tokio::io::{AsyncBufReadExt as _, BufReader};

    const LIVE_CANDIDATE_BUDGET: Duration = Duration::from_secs(60);

    struct Fixture {
        root: PathBuf,
        directory: PathBuf,
        selected: PathBuf,
        writer: KernelWriter,
    }

    impl Fixture {
        fn new() -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "vessel-descriptor-{}-{:?}-{stamp}",
                std::process::id(),
                std::thread::current().id()
            ));
            std::fs::create_dir(&root).unwrap();
            let process = NodeProcess(std::process::id());
            let directory = root.join(process.0.to_string());
            std::fs::create_dir_all(directory.join("fd")).unwrap();
            std::fs::create_dir_all(directory.join("fdinfo")).unwrap();
            let selected = root.join("selected");
            std::fs::write(&selected, b"owned fixture").unwrap();
            let writer = KernelWriter {
                process,
                file: identity(&selected).unwrap(),
            };
            Self {
                root,
                directory,
                selected,
                writer,
            }
        }

        fn add(&self, name: &str, target: &Path, bytes: &[u8]) {
            std::os::unix::fs::symlink(target, self.directory.join("fd").join(name)).unwrap();
            std::fs::write(self.directory.join("fdinfo").join(name), bytes).unwrap();
        }

        fn line(&self, process: NodeProcess) -> Vec<u8> {
            let file = &self.writer.file;
            format!(
                "lock:\t1: FLOCK ADVISORY WRITE {} {:02x}:{:02x}:{} 0 EOF\n",
                process.0, file.major, file.minor, file.inode
            )
            .into_bytes()
        }

        fn scan(&self) -> io::Result<Possession> {
            DescriptorScan::open(&self.root, self.writer.process)?.find(&self.writer)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            if let Err(error) = std::fs::remove_dir_all(&self.root) {
                assert!(
                    std::thread::panicking(),
                    "cannot clean descriptor fixture: {error}"
                );
            }
        }
    }

    #[test]
    fn foreign_descriptor_contents_cannot_hide_the_selected_lock() {
        let fixture = Fixture::new();
        let other = fixture.root.join("other");
        std::fs::write(&other, b"other file").unwrap();
        let too_long = usize::try_from(Record::MAX_BYTES.saturating_add(1)).unwrap();
        fixture.add("0", &other, &vec![b'\xff'; too_long]);
        fixture.add(
            "1",
            &fixture.selected,
            &fixture.line(fixture.writer.process),
        );
        assert_eq!(fixture.scan().unwrap().descriptor, Descriptor(1));
        assert_eq!(std::fs::read(&fixture.selected).unwrap(), b"owned fixture");
    }

    #[test]
    fn an_inode_match_without_its_lock_record_is_no_possession_proof() {
        let fixture = Fixture::new();
        fixture.add("0", &fixture.selected, b"pos:\t0\nflags:\t0100002\n");
        let error = fixture.scan().unwrap_err();
        assert!(matches!(
            error.get_ref().and_then(|e| e.downcast_ref::<Error>()),
            Some(Error::Missing)
        ));
    }

    #[test]
    fn wrong_process_and_duplicate_selected_lock_records_are_refused() {
        let fixture = Fixture::new();
        let wrong = NodeProcess(fixture.writer.process.0.saturating_add(1));
        fixture.add("0", &fixture.selected, &fixture.line(wrong));
        let error = fixture.scan().unwrap_err();
        assert!(matches!(
            error.get_ref().and_then(|e| e.downcast_ref::<Error>()),
            Some(Error::DifferentProcess)
        ));
        let mut doubled = fixture.line(fixture.writer.process);
        doubled.extend(fixture.line(fixture.writer.process));
        std::fs::write(fixture.directory.join("fdinfo/0"), doubled).unwrap();
        let error = fixture.scan().unwrap_err();
        assert!(matches!(
            error.get_ref().and_then(|e| e.downcast_ref::<Error>()),
            Some(Error::Multiple)
        ));
    }

    #[test]
    fn the_descriptor_count_and_its_message_share_the_same_bound() {
        let fixture = Fixture::new();
        let other = fixture.root.join("other");
        std::fs::write(&other, b"not selected").unwrap();
        for number in 0..=DescriptorScan::MAX_ENTRIES {
            fixture.add(&number.to_string(), &other, b"");
        }
        let error = fixture.scan().unwrap_err();
        assert!(matches!(
            error.get_ref().and_then(|e| e.downcast_ref::<Error>()),
            Some(Error::TooMany)
        ));
        assert!(
            error
                .to_string()
                .contains(&DescriptorScan::MAX_ENTRIES.to_string())
        );
    }

    #[test]
    fn replacing_a_numeric_directory_path_does_not_redirect_the_held_reference() {
        let fixture = Fixture::new();
        fixture.add(
            "0",
            &fixture.selected,
            &fixture.line(fixture.writer.process),
        );
        let scan = DescriptorScan::open(&fixture.root, fixture.writer.process).unwrap();
        std::fs::rename(&fixture.directory, fixture.root.join("previous")).unwrap();
        std::fs::create_dir_all(fixture.directory.join("fd")).unwrap();
        assert_eq!(
            scan.find(&fixture.writer).unwrap().descriptor,
            Descriptor(0)
        );
        // This is ordinary directory anchoring, not a simulation proving
        // numeric PID reservation or process generation/namespace identity.
        assert!(matches!(
            fixture
                .scan()
                .unwrap_err()
                .get_ref()
                .and_then(|e| e.downcast_ref::<Error>()),
            Some(Error::Missing)
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_unrelated_live_candidate_lacking_the_locked_description_is_refused() {
        let checked = vessel_testkit::within(LIVE_CANDIDATE_BUDGET, async {
            let fixture = Fixture::new();
            let held = File::options()
                .read(true)
                .write(true)
                .open(&fixture.selected)
                .unwrap();
            held.try_lock().unwrap();
            let fake = vessel_testkit::Fake::Tui.path().unwrap();
            let crate::Spawned {
                mut child,
                stdin,
                stdout,
                stderr,
            } = crate::spawn(&crate::CommandSpec::new(fake)).unwrap();
            let mut lines = BufReader::new(stdout).lines();
            loop {
                let line = lines
                    .next_line()
                    .await
                    .unwrap()
                    .expect("fake ended before readiness");
                if line == "ready" {
                    break;
                }
            }
            let selected = KernelWriter {
                process: NodeProcess(child.pid()),
                file: fixture.writer.file.clone(),
            };
            let reference = Directory::open(Path::new("/proc"), selected.process).unwrap();
            let generation =
                crate::lock::generation::Proof::capture(&reference.path(), selected.process)
                    .unwrap();
            generation.confirm(&reference.path()).unwrap();
            let error = Proof::capture(&selected).unwrap_err();
            assert!(matches!(
                error.get_ref().and_then(|e| e.downcast_ref::<Error>()),
                Some(Error::Missing)
            ));
            let other = File::open(&fixture.selected).unwrap();
            assert!(matches!(
                other.try_lock(),
                Err(std::fs::TryLockError::WouldBlock)
            ));
            // EOF ends only this owned fake; its existing Child guard sweeps
            // its fake grandchild. No PID from the observation is signalled.
            drop(stdin);
            assert_eq!(child.wait().await.unwrap(), Some(0));
            let gone = generation.confirm(&reference.path()).unwrap_err();
            assert!(
                gone.kind() == io::ErrorKind::NotFound
                    || gone.raw_os_error()
                        == io::Error::from(nix::errno::Errno::ESRCH).raw_os_error(),
                "a retained reference to the exited fake must refuse: {gone}"
            );
            // Actual process disappearance is checked here; reusing its
            // numeric PID is not simulated or inferred from this case.
            drop(stderr);
            held.unlock().unwrap();
        })
        .await;
        assert!(
            checked.is_ok(),
            "live candidate fixture exceeded {LIVE_CANDIDATE_BUDGET:?}: {checked:?}"
        );
    }
}
