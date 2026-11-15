//! Numeric facts about this process only; never arbitrary pid/path inspection.
//! This module alone decodes Linux's fixed /proc/self/status format. Raw
//! names, paths and lines never escape the edge. Callers run this synchronous
//! filesystem operation on a blocking worker and retain their work lease.
//!
//! [Decision Log]
//! Standard Rust supplies the current pid but no portable resident memory
//! query. Reading this process's bounded kernel metadata needs no child,
//! credentials or environment. Other targets explicitly refuse until an
//! owned safe source exists; no shell command or arbitrary pid/path is used.
use crate::{InvalidProcessId, ProcessId};
#[cfg(any(target_os = "linux", test))]
use std::io::Read as _;
use std::{
    io,
    num::{ParseIntError, TryFromIntError},
};

/// Maximum bytes read from the fixed self-status source.
pub const MAX_STATUS_BYTES: usize = 16 * 1024;
/// Minimum valid process id.
pub const MIN_PROCESS_ID: u32 = ProcessId::MIN.get();
/// Maximum representable kernel process id.
pub const MAX_PROCESS_ID: u32 = u32::MAX;
#[cfg(any(target_os = "linux", test))]
const BYTES_PER_KIB: u64 = 1024;

/// A checked byte count from kernel memory metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryBytes(u64);
/// Resident and virtual memory observed together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Memory {
    /// Resident memory in bytes.
    pub resident: MemoryBytes,
    /// Virtual address space in bytes.
    pub virtual_size: MemoryBytes,
}
/// Missing kernel memory fields are explicit, never healthy zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryObservation {
    /// Both memory fields were present.
    Available(Memory),
    /// The kernel omitted both memory fields.
    NotProvided,
}
/// Allowlisted self-process metadata without names or paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Snapshot {
    /// Identity of this process.
    pub pid: ProcessId,
    /// Parent identity, if the kernel reports one.
    pub parent: Option<ProcessId>,
    /// Memory availability at this observation.
    pub memory: MemoryObservation,
}
/// The numeric kernel field that failed decoding.
#[derive(Debug, Clone, Copy)]
pub enum Field {
    /// Process id.
    Pid,
    /// Parent process id.
    Parent,
    /// Resident bytes.
    Resident,
    /// Virtual bytes.
    Virtual,
}
impl std::fmt::Display for Field {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(match self {
            Self::Pid => "process id",
            Self::Parent => "parent process id",
            Self::Resident => "resident bytes",
            Self::Virtual => "virtual bytes",
        })
    }
}
/// Typed self-process read/decode failure, without raw source lines.
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    /// A typed source or contract failure, without source contents.
    #[error("self-process metrics are not provided on this platform")]
    Unsupported,
    /// A typed source or contract failure, without source contents.
    #[error("could not read self-process metadata")]
    Read(#[from] io::Error),
    /// A typed source or contract failure, without source contents.
    #[error("self-process metadata exceeds {MAX_STATUS_BYTES} bytes")]
    TooLarge,
    /// A typed source or contract failure, without source contents.
    #[error("self-process metadata is not valid UTF-8")]
    Encoding(#[from] std::str::Utf8Error),
    /// A typed source or contract failure, without source contents.
    #[error("self-process metadata field is missing: {0}")]
    Missing(Field),
    /// A typed source or contract failure, without source contents.
    #[error("self-process metadata field is repeated: {0}")]
    Duplicate(Field),
    /// A typed source or contract failure, without source contents.
    #[error("self-process metadata has an invalid number for {field}")]
    Number {
        /// The allowlisted field category.
        field: Field,
        /// The typed parser/conversion cause contains no input string.
        #[source]
        source: ParseIntError,
    },
    /// A typed source or contract failure, without source contents.
    #[error("{field} must be at most {MAX_PROCESS_ID}")]
    IdRange {
        /// The allowlisted field category.
        field: Field,
        /// The typed parser/conversion cause contains no input string.
        #[source]
        source: TryFromIntError,
    },
    /// A typed source or contract failure, without source contents.
    #[error("{field} must be at least {MIN_PROCESS_ID}")]
    ZeroId {
        /// The allowlisted field category.
        field: Field,
        /// The canonical process-id constructor's typed failure.
        #[source]
        source: InvalidProcessId,
    },
    /// A typed source or contract failure, without source contents.
    #[error("self-process memory has an invalid unit for {0}")]
    Unit(Field),
    /// A typed source or contract failure, without source contents.
    #[error("self-process memory conversion overflowed for {0}")]
    Overflow(Field),
    /// A typed source or contract failure, without source contents.
    #[error("self-process memory has an incomplete pair of fields")]
    IncompleteMemory,
    /// A typed source or contract failure, without source contents.
    #[error("the process metadata does not describe this process")]
    WrongProcess,
    /// A typed source or contract failure, without source contents.
    #[error("the process-metadata read limit is not representable")]
    Limit(#[source] TryFromIntError),
}
#[cfg(any(target_os = "linux", test))]
fn id(raw: &str, field: Field) -> Result<ProcessId, SnapshotError> {
    let parsed: u64 = raw
        .parse()
        .map_err(|source| SnapshotError::Number { field, source })?;
    let parsed =
        u32::try_from(parsed).map_err(|source| SnapshotError::IdRange { field, source })?;
    ProcessId::try_from(parsed).map_err(|source| SnapshotError::ZeroId { field, source })
}
#[cfg(any(target_os = "linux", test))]
fn parent(raw: &str) -> Result<Option<ProcessId>, SnapshotError> {
    // The kernel format uses zero for no parent. It never escapes as a pid.
    if raw == "0" {
        return Ok(None);
    }
    id(raw, Field::Parent).map(Some)
}
#[cfg(any(target_os = "linux", test))]
fn memory(raw: &str, field: Field) -> Result<MemoryBytes, SnapshotError> {
    let mut parts = raw.split_whitespace();
    let number = parts.next().ok_or(SnapshotError::Missing(field))?;
    if parts.next() != Some("kB") || parts.next().is_some() {
        return Err(SnapshotError::Unit(field));
    }
    let kib: u64 = number
        .parse()
        .map_err(|source| SnapshotError::Number { field, source })?;
    kib.checked_mul(BYTES_PER_KIB)
        .map(MemoryBytes)
        .ok_or(SnapshotError::Overflow(field))
}
#[cfg(any(target_os = "linux", test))]
fn unique<T>(slot: &mut Option<T>, value: T, field: Field) -> Result<(), SnapshotError> {
    if slot.is_some() {
        return Err(SnapshotError::Duplicate(field));
    }
    *slot = Some(value);
    Ok(())
}
#[cfg(any(target_os = "linux", test))]
fn decode(reader: impl io::Read, expected: ProcessId) -> Result<Snapshot, SnapshotError> {
    let limit = u64::try_from(MAX_STATUS_BYTES)
        .map_err(SnapshotError::Limit)?
        .saturating_add(1);
    let mut bytes = Vec::new();
    let _read_bytes = reader.take(limit).read_to_end(&mut bytes)?;
    if bytes.len() > MAX_STATUS_BYTES {
        return Err(SnapshotError::TooLarge);
    }
    let text = std::str::from_utf8(&bytes)?;
    let (mut pid, mut parent_id, mut rss, mut virtual_size) = (None, None, None, None);
    for line in text.lines() {
        let Some((key, raw)) = line.split_once(':') else {
            continue;
        };
        let raw = raw.trim();
        // Raw kernel field labels are decoded only at this format edge.
        match key {
            "Pid" => unique(&mut pid, id(raw, Field::Pid)?, Field::Pid)?,
            "PPid" => unique(&mut parent_id, parent(raw)?, Field::Parent)?,
            "VmRSS" => unique(&mut rss, memory(raw, Field::Resident)?, Field::Resident)?,
            "VmSize" => unique(
                &mut virtual_size,
                memory(raw, Field::Virtual)?,
                Field::Virtual,
            )?,
            _other_kernel_field => {}
        }
    }
    let pid = pid.ok_or(SnapshotError::Missing(Field::Pid))?;
    let parent = parent_id.ok_or(SnapshotError::Missing(Field::Parent))?;
    if pid != expected {
        return Err(SnapshotError::WrongProcess);
    }
    let memory = match (rss, virtual_size) {
        (Some(resident), Some(virtual_size)) => MemoryObservation::Available(Memory {
            resident,
            virtual_size,
        }),
        (None, None) => MemoryObservation::NotProvided,
        (Some(_), None) | (None, Some(_)) => return Err(SnapshotError::IncompleteMemory),
    };
    Ok(Snapshot {
        pid,
        parent,
        memory,
    })
}
/// Read this process only. Blocks on bounded kernel metadata I/O.
///
/// # Errors
/// Read, size, decoding and consistency failures; raw source lines are omitted.
#[cfg(target_os = "linux")]
pub fn observation() -> Result<Snapshot, SnapshotError> {
    let pid = ProcessId::try_from(std::process::id()).map_err(|source| SnapshotError::ZeroId {
        field: Field::Pid,
        source,
    })?;
    decode(std::fs::File::open("/proc/self/status")?, pid)
}
/// This target has no owned self-process source yet.
///
/// # Errors
/// Always returns the explicit Unsupported variant.
#[cfg(not(target_os = "linux"))]
pub fn observation() -> Result<Snapshot, SnapshotError> {
    Err(SnapshotError::Unsupported)
}

impl MemoryBytes {
    /// Checked bytes; never a parsed unit string.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const PRIVATE: &str = "PRIVATE-PROCESS-DIAGNOSTIC-CANARY";
    fn expected() -> ProcessId {
        ProcessId::try_from(42).unwrap()
    }
    fn input(extra: &str) -> String {
        format!("Name:\t{PRIVATE}\nPid:\t42\nPPid:\t7\n{extra}")
    }
    #[test]
    fn returns_only_identity_and_typed_memory_without_private_fields() {
        let source = input("VmRSS:\t3 kB\nVmSize:\t9 kB\n");
        let got = decode(source.as_bytes(), expected()).unwrap();
        assert_eq!(got.pid, expected());
        assert_eq!(got.parent, Some(ProcessId::try_from(7).unwrap()));
        assert_eq!(
            got.memory,
            MemoryObservation::Available(Memory {
                resident: MemoryBytes(3_u64.checked_mul(BYTES_PER_KIB).unwrap()),
                virtual_size: MemoryBytes(9_u64.checked_mul(BYTES_PER_KIB).unwrap()),
            })
        );
        assert!(!format!("{got:?}").contains(PRIVATE));
    }
    #[test]
    fn unavailable_memory_and_no_parent_are_not_healthy_zero_metrics() {
        let got = decode(b"Pid: 42\nPPid: 0\n".as_slice(), expected()).unwrap();
        assert_eq!(got.parent, None);
        assert_eq!(got.memory, MemoryObservation::NotProvided);
        assert!(matches!(
            decode(input("VmRSS: 0 kB\n").as_bytes(), expected()),
            Err(SnapshotError::IncompleteMemory)
        ));
    }
    #[test]
    fn malformed_numbers_units_duplicates_and_wrong_identity_refuse() {
        for raw in [
            format!("Pid: {PRIVATE}\nPPid: 7\n"),
            input(&format!("VmRSS: 1 {PRIVATE}\nVmSize: 2 kB\n")),
            input("Pid: 42\n"),
        ] {
            let error = decode(raw.as_bytes(), expected()).unwrap_err();
            assert!(!error.to_string().contains(PRIVATE));
            assert!(!format!("{error:?}").contains(PRIVATE));
        }
        assert!(matches!(
            decode(b"Pid: 41\nPPid: 7\n".as_slice(), expected()),
            Err(SnapshotError::WrongProcess)
        ));
        assert!(matches!(
            decode(b"Pid: 0\nPPid: 7\n".as_slice(), expected()),
            Err(SnapshotError::ZeroId {
                field: Field::Pid,
                ..
            })
        ));
        assert!(matches!(
            decode(b"PPid: 7\n".as_slice(), expected()),
            Err(SnapshotError::Missing(Field::Pid))
        ));
    }
    #[test]
    fn bound_is_checked_before_decoding_and_overflow_keeps_a_typed_failure() {
        let source = vec![b'x'; MAX_STATUS_BYTES.saturating_add(1)];
        let error = decode(source.as_slice(), expected()).unwrap_err();
        assert!(matches!(error, SnapshotError::TooLarge));
        assert!(error.to_string().contains(&MAX_STATUS_BYTES.to_string()));
        assert!(matches!(
            memory(&format!("{} kB", u64::MAX), Field::Resident),
            Err(SnapshotError::Overflow(Field::Resident))
        ));
        let error = id(&u64::MAX.to_string(), Field::Pid).unwrap_err();
        assert!(matches!(error, SnapshotError::IdRange { .. }));
        assert!(error.to_string().contains(&MAX_PROCESS_ID.to_string()));
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn real_self_metadata_matches_the_test_process_without_running_a_child() {
        let got = observation().unwrap();
        assert_eq!(got.pid.get(), std::process::id());
        assert!(matches!(got.memory, MemoryObservation::Available(_)));
    }
    #[cfg(not(target_os = "linux"))]
    #[test]
    fn unsupported_platforms_do_not_run_a_fallback_command() {
        assert!(matches!(observation(), Err(SnapshotError::Unsupported)));
    }
}
