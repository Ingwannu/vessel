//! Bind descriptor evidence to one process generation and one PID namespace.
//! Observations refuse ambiguity; they do not reserve a PID or authorize signals.

use std::fs::File;
use std::io;
use std::os::unix::fs::MetadataExt as _;
use std::path::Path;
use std::sync::Arc;

use super::NodeProcess;
use crate::proc_stat::{StartTicks, Stat};
use crate::procfs::Record;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Generation {
    process: NodeProcess,
    started: StartTicks,
}

impl Generation {
    // Fixture aliases preserve the old test text while the parser owns layout.
    #[cfg(test)]
    const FIRST_AFTER_COMM: usize = Stat::FIRST_AFTER_COMM;
    #[cfg(test)]
    const START_TICKS: usize = Stat::START_TICKS;

    fn read(path: &Path) -> io::Result<Self> {
        let record = Record::read(&path.join("stat"))?;
        Self::decode(record.bytes()).map_err(io::Error::other)
    }

    fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let record = Stat::split(bytes).map_err(Error::from)?;
        let process = process(record.process())?;
        let started = record.started().map_err(Error::from)?;
        Ok(Self { process, started })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NamespaceIdentity {
    device: u64,
    inode: u64,
}

#[derive(Debug)]
struct Namespace(File);

impl Namespace {
    fn open(path: &Path) -> io::Result<Arc<Self>> {
        File::open(path).map(Self).map(Arc::new)
    }

    fn identity(&self) -> io::Result<NamespaceIdentity> {
        let metadata = self.0.metadata()?;
        Ok(NamespaceIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }
}

#[derive(Debug, Clone)]
struct NamespaceProof {
    caller: Arc<Namespace>,
    candidate: Arc<Namespace>,
    identity: NamespaceIdentity,
}

impl PartialEq for NamespaceProof {
    fn eq(&self, other: &Self) -> bool {
        // Namespace file handles retain lifetime; identity, not handle number,
        // determines whether two observations name the same namespace.
        self.identity == other.identity
    }
}
impl Eq for NamespaceProof {}

impl NamespaceProof {
    fn capture(candidate: &Path) -> io::Result<Self> {
        let own = Record::read(Path::new("/proc/self/status"))?;
        same_view(own.bytes(), NodeProcess(std::process::id())).map_err(io::Error::other)?;
        let caller = Namespace::open(Path::new("/proc/self/ns/pid"))?;
        let candidate = Namespace::open(&candidate.join("ns/pid"))?;
        Self::from_references(caller, candidate)
    }

    fn from_references(caller: Arc<Namespace>, candidate: Arc<Namespace>) -> io::Result<Self> {
        let identity = caller.identity()?;
        if identity != candidate.identity()? {
            return Err(io::Error::other(Error::Namespace));
        }
        Ok(Self {
            caller,
            candidate,
            identity,
        })
    }

    fn retained(&self) -> io::Result<()> {
        if self.caller.identity()? != self.identity || self.candidate.identity()? != self.identity {
            return Err(io::Error::other(Error::Namespace));
        }
        Ok(())
    }
}

/// Keep the generation and namespace references that bracket descriptor capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Proof {
    generation: Generation,
    namespace: NamespaceProof,
}

impl Proof {
    /// Begin a bounded observation; no numeric PID is reserved by this proof.
    ///
    /// # Errors
    /// Missing, malformed or mismatched generation/namespace data refuses capture.
    pub(super) fn capture(path: &Path, expected: NodeProcess) -> io::Result<Self> {
        let generation = Generation::read(path)?;
        if generation.process != expected {
            return Err(io::Error::other(Error::Process));
        }
        Ok(Self {
            generation,
            namespace: NamespaceProof::capture(path)?,
        })
    }

    /// Finish the observation with the same generation and namespace evidence.
    ///
    /// # Errors
    /// A changed or inaccessible process/namespace refuses the whole observation.
    pub(super) fn confirm(&self, path: &Path) -> io::Result<()> {
        self.namespace.retained()?;
        let current = Self::capture(path, self.generation.process)?;
        if current != *self {
            return Err(io::Error::other(Error::Changed));
        }
        Ok(())
    }
}

fn same_view(bytes: &[u8], expected: NodeProcess) -> Result<(), Error> {
    let mut found = false;
    for line in bytes.split(|byte| *byte == b'\n') {
        let Some(value) = line.strip_prefix(b"NStgid:") else {
            continue;
        };
        if found {
            return Err(Error::Hierarchy);
        }
        found = true;
        let mut values = value
            .split(u8::is_ascii_whitespace)
            .filter(|v| !v.is_empty());
        let observed = process(values.next().ok_or(Error::Hierarchy)?)?;
        // One hierarchy entry proves the proc mount and this active PID
        // namespace have the same level. Numeric equality alone does not.
        if observed != expected || values.next().is_some() {
            return Err(Error::Hierarchy);
        }
    }
    if found { Ok(()) } else { Err(Error::Hierarchy) }
}

fn process(bytes: &[u8]) -> Result<NodeProcess, Error> {
    let value: u32 = number(bytes)?;
    if value < NodeProcess::MINIMUM {
        return Err(Error::Reserved);
    }
    Ok(NodeProcess(value))
}

fn number<T: std::str::FromStr<Err = std::num::ParseIntError>>(bytes: &[u8]) -> Result<T, Error> {
    let text = std::str::from_utf8(bytes).map_err(Error::Encoding)?;
    text.parse().map_err(Error::Number)
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("process generation record lacks a required field")]
    Stat,
    #[error("process observation number is not valid Unicode")]
    Encoding(#[source] std::str::Utf8Error),
    #[error("process observation number cannot be decoded")]
    Number(#[source] std::num::ParseIntError),
    #[error("process identifier must be at least {}", NodeProcess::MINIMUM)]
    Reserved,
    #[error("process generation names another process")]
    Process,
    #[error("proc PID namespace hierarchy is missing or does not match this process")]
    Hierarchy,
    #[error("candidate PID namespace differs from the observer")]
    Namespace,
    #[error("process generation or namespace changed during observation")]
    Changed,
}

impl From<crate::proc_stat::Error> for Error {
    fn from(error: crate::proc_stat::Error) -> Self {
        match error {
            crate::proc_stat::Error::Fields => Self::Stat,
            crate::proc_stat::Error::Encoding(cause) => Self::Encoding(cause),
            crate::proc_stat::Error::Number(cause) => Self::Number(cause),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stat(process: NodeProcess, comm: &[u8], ticks: &[u8]) -> Vec<u8> {
        let mut bytes = format!("{} (", process.0).into_bytes();
        bytes.extend_from_slice(comm);
        bytes.extend_from_slice(b") S");
        for _field in Generation::FIRST_AFTER_COMM.saturating_add(1)..Generation::START_TICKS {
            bytes.extend_from_slice(b" 0");
        }
        bytes.push(b' ');
        bytes.extend_from_slice(ticks);
        bytes
    }

    #[test]
    fn generation_keeps_command_bytes_opaque_and_decodes_only_its_numbers() {
        let pid = NodeProcess(NodeProcess::MINIMUM);
        for comm in [
            b"normal".as_slice(),
            b"space and ) (parentheses)",
            b"invalid \xff\n ) bytes",
        ] {
            assert_eq!(
                Generation::decode(&stat(pid, comm, b"781")).unwrap(),
                Generation {
                    process: pid,
                    started: StartTicks(781)
                }
            );
        }
        assert!(matches!(
            Generation::decode(&stat(pid, b"private-command", b"bad")),
            Err(Error::Number(_))
        ));
        assert!(matches!(
            Generation::decode(b"bad (private-command)"),
            Err(Error::Number(_))
        ));
        assert!(matches!(
            Generation::decode(format!("{} missing-parentheses", NodeProcess::MINIMUM).as_bytes()),
            Err(Error::Stat)
        ));
        assert!(matches!(
            Generation::decode(&stat(pid, b"private-command", b"")),
            Err(Error::Stat)
        ));
        let reserved = NodeProcess(pid.0.saturating_sub(1));
        let error = Generation::decode(&stat(reserved, b"private-command", b"0")).unwrap_err();
        assert!(matches!(error, Error::Reserved));
        assert!(
            error
                .to_string()
                .contains(&NodeProcess::MINIMUM.to_string())
        );
        assert!(!error.to_string().contains("private-command"));
    }

    #[test]
    fn the_recorded_stat_layout_pins_the_start_tick_field_independently() {
        let bytes = b"77 (command ) with spaces) S 101 102 103 104 105 106 107 108 109 110 111 112 113 114 115 116 117 118 7654321 909 910\n";
        assert_eq!(
            Generation::decode(bytes).unwrap(),
            Generation {
                process: NodeProcess(77),
                started: StartTicks(7_654_321)
            }
        );
        let malformed = b"77 (command)S 101 102 103 104 105 106 107 108 109 110 111 112 113 114 115 116 117 118 7654321";
        assert!(matches!(Generation::decode(malformed), Err(Error::Stat)));
    }

    #[test]
    fn numeric_pid_equality_without_one_namespace_level_is_insufficient() {
        let pid = NodeProcess(std::process::id());
        let one = format!("Name:	private-command\nNStgid:	{}\n", pid.0);
        assert!(same_view(one.as_bytes(), pid).is_ok());
        let repeated = format!("{one}NStgid:	{}\n", pid.0);
        let nested = format!("NStgid:	{}\t{}\n", pid.0, pid.0);
        let other = format!("NStgid:	{}\n", pid.0.saturating_add(1));
        for bytes in [
            b"Name:	private-command\n".as_slice(),
            b"NStgid:	\n",
            repeated.as_bytes(),
            nested.as_bytes(),
            other.as_bytes(),
        ] {
            assert!(matches!(same_view(bytes, pid), Err(Error::Hierarchy)));
        }
        assert!(matches!(
            same_view(b"NStgid:	invalid\n", pid),
            Err(Error::Number(_))
        ));
    }

    #[test]
    fn namespaces_are_compared_by_retained_file_identity_not_number_text() {
        let caller = Namespace::open(Path::new("/proc/self/ns/pid")).unwrap();
        let same = Namespace::open(Path::new("/proc/self/ns/pid")).unwrap();
        let proof = NamespaceProof::from_references(Arc::clone(&caller), same).unwrap();
        proof.retained().unwrap();
        let different = Namespace::open(Path::new("/proc/self/ns/mnt")).unwrap();
        let error = NamespaceProof::from_references(caller, different).unwrap_err();
        assert!(matches!(
            error.get_ref().and_then(|e| e.downcast_ref::<Error>()),
            Some(Error::Namespace)
        ));
    }

    #[test]
    fn generation_is_checked_again_after_the_observation() {
        let pid = NodeProcess(std::process::id());
        let path = Path::new("/proc/self");
        let mut proof = Proof::capture(path, pid).unwrap();
        proof.confirm(path).unwrap();
        proof.generation.started.0 = proof.generation.started.0.saturating_add(1);
        let error = proof.confirm(path).unwrap_err();
        assert!(matches!(
            error.get_ref().and_then(|e| e.downcast_ref::<Error>()),
            Some(Error::Changed)
        ));
        let error = Proof::capture(path, NodeProcess(pid.0.saturating_add(1))).unwrap_err();
        assert!(matches!(
            error.get_ref().and_then(|e| e.downcast_ref::<Error>()),
            Some(Error::Process)
        ));
        let error =
            Generation::read(Path::new("/proc/self/absent-observation-fixture")).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }
}
