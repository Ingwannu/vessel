//! Linux exposes flock writers by file identity without acquiring the lock.
//! Keeping this edge beside `NodeLock` avoids a second lock parser in the CLI.

use super::{KernelLock, LockFileIdentity, NodeProcess};
use std::io::{self, Read};
use std::path::Path;

struct LockTable(String);

impl LockTable {
    const MAX_BYTES: u64 = 16_777_216;

    fn read() -> io::Result<Self> {
        Self::read_from(std::fs::File::open("/proc/locks")?)
    }

    fn read_from(input: impl Read) -> io::Result<Self> {
        let mut text = String::new();
        let count = input
            .take(Self::MAX_BYTES.saturating_add(1))
            .read_to_string(&mut text)?;
        let maximum = usize::try_from(Self::MAX_BYTES)
            .map_err(DecodeError::Length)
            .map_err(io::Error::other)?;
        if count > maximum {
            return Err(io::Error::other(DecodeError::TooLong));
        }
        Ok(Self(text))
    }

    fn writer(&self, file: &LockFileIdentity) -> Result<Option<KernelLock>, DecodeError> {
        let mut found = None;
        for line in self.0.lines() {
            let Some(writer) = writer(line, file)? else {
                continue;
            };
            if found.replace(writer).is_some() {
                return Err(DecodeError::Multiple);
            }
        }
        Ok(found)
    }
}

#[derive(Debug, thiserror::Error)]
enum DecodeError {
    #[error("kernel lock record lacks a required field")]
    Missing,
    #[error("kernel lock record has an invalid number")]
    Number(#[source] std::num::ParseIntError),
    #[error("kernel lock device identity contains extra fields")]
    Extra,
    #[error("kernel lock table exceeds {} bytes", LockTable::MAX_BYTES)]
    TooLong,
    #[error("kernel lock table length cannot be represented")]
    Length(#[source] std::num::TryFromIntError),
    #[error("kernel lock PID must be at least {}", NodeProcess::MINIMUM)]
    Pid,
    #[error("more than one exclusive kernel writer owns the node lock")]
    Multiple,
    #[error("kernel device identity cannot be decoded")]
    Device,
}

/// Only `NodeLock` may expose this kernel snapshot as process authority.
///
/// # Errors
/// The lock metadata or bounded procfs table cannot be read/decoded.
pub(super) fn observe(path: &Path) -> io::Result<Option<KernelLock>> {
    let file = match identity(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    LockTable::read()?.writer(&file).map_err(io::Error::other)
}

fn identity(path: &Path) -> io::Result<LockFileIdentity> {
    use std::os::unix::fs::MetadataExt as _;
    let metadata = std::fs::metadata(path)?;
    let device = metadata.dev();
    // Linux dev_t separates high device bits. A low-byte approximation
    // would identify a different mount's inode as this directory's lock.
    let major = (device & 0x0000_0000_000f_ff00)
        .checked_shr(8)
        .ok_or_else(|| io::Error::other(DecodeError::Device))?
        | (device & 0xffff_f000_0000_0000)
            .checked_shr(32)
            .ok_or_else(|| io::Error::other(DecodeError::Device))?;
    let minor = (device & 0xff)
        | (device & 0x0000_0fff_fff0_0000)
            .checked_shr(12)
            .ok_or_else(|| io::Error::other(DecodeError::Device))?;
    Ok(LockFileIdentity {
        major,
        minor,
        inode: metadata.ino(),
    })
}

fn writer(line: &str, selected: &LockFileIdentity) -> Result<Option<KernelLock>, DecodeError> {
    let mut fields = line.split_whitespace();
    let _sequence = fields.next();
    // A blocked waiter, a POSIX range or a read lock is no exclusive flock.
    if fields.next() != Some("FLOCK")
        || fields.next() != Some("ADVISORY")
        || fields.next() != Some("WRITE")
    {
        return Ok(None);
    }
    // Other files are outside this query's authority. Do not let their PID
    // spelling or an unrecognizable identity hide a matching writer later.
    let Some((process, identity)) = fields.next().zip(fields.next()) else {
        return Ok(None);
    };
    let file = match parse_identity(identity) {
        Ok(file) => file,
        Err(error) => {
            tracing::debug!(%error, "kernel lock row has no recognizable file identity");
            return Ok(None);
        }
    };
    if &file != selected {
        return Ok(None);
    }
    let process = process.parse().map_err(DecodeError::Number)?;
    let writer = KernelLock {
        process: NodeProcess(process),
        file,
    };
    if writer.process.0 < NodeProcess::MINIMUM {
        return Err(DecodeError::Pid);
    }
    Ok(Some(writer))
}

fn parse_identity(text: &str) -> Result<LockFileIdentity, DecodeError> {
    let mut fields = text.split(':');
    let major = u64::from_str_radix(fields.next().ok_or(DecodeError::Missing)?, 16)
        .map_err(DecodeError::Number)?;
    let minor = u64::from_str_radix(fields.next().ok_or(DecodeError::Missing)?, 16)
        .map_err(DecodeError::Number)?;
    let inode = fields
        .next()
        .ok_or(DecodeError::Missing)?
        .parse()
        .map_err(DecodeError::Number)?;
    if fields.next().is_some() {
        return Err(DecodeError::Extra);
    }
    Ok(LockFileIdentity {
        major,
        minor,
        inode,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_exclusive_writer_supplies_a_process_and_device_identity() {
        let file = parse_identity("00:1a:765").unwrap();
        let observed = writer("42: FLOCK ADVISORY WRITE 8123 00:1a:765 0 EOF", &file)
            .unwrap()
            .unwrap();
        assert_eq!(observed.process, NodeProcess(8123));
        assert_eq!(
            observed.file,
            LockFileIdentity {
                major: 0,
                minor: 26,
                inode: 765
            }
        );
        for line in [
            "42: -> FLOCK ADVISORY WRITE 8123 00:1a:765 0 EOF",
            "42: POSIX ADVISORY WRITE 8123 00:1a:765 0 EOF",
            "42: FLOCK ADVISORY READ 8123 00:1a:765 0 EOF",
        ] {
            assert!(writer(line, &file).unwrap().is_none());
        }
        assert!(matches!(
            writer("42: FLOCK ADVISORY WRITE abc 00:1a:765", &file),
            Err(DecodeError::Number(_))
        ));
        assert!(matches!(
            parse_identity("00:1a:765:4"),
            Err(DecodeError::Extra)
        ));
        assert!(writer("42: FLOCK ADVISORY WRITE", &file).unwrap().is_none());
        let reserved = NodeProcess::MINIMUM.saturating_sub(1);
        assert!(matches!(
            writer(
                &format!("42: FLOCK ADVISORY WRITE {reserved} 00:1a:765"),
                &file
            ),
            Err(DecodeError::Pid)
        ));
    }

    #[test]
    fn the_allocation_limit_and_its_error_come_from_the_same_bound() {
        let Err(error) = LockTable::read_from(io::repeat(b'x')) else {
            panic!("accepted an unlimited lock table");
        };
        assert!(matches!(
            error
                .get_ref()
                .and_then(|source| source.downcast_ref::<DecodeError>()),
            Some(DecodeError::TooLong)
        ));
        assert!(
            error
                .to_string()
                .contains(&LockTable::MAX_BYTES.to_string())
        );
    }

    #[test]
    fn two_exclusive_writers_are_not_silently_reduced_to_one() {
        let table = LockTable("42: FLOCK ADVISORY WRITE 8123 00:1a:765 0 EOF\n43: FLOCK ADVISORY WRITE 8124 00:1a:765 0 EOF".to_owned());
        let file = parse_identity("00:1a:765").unwrap();
        assert!(matches!(table.writer(&file), Err(DecodeError::Multiple)));
    }

    #[test]
    fn undecodable_foreign_rows_cannot_hide_a_selected_writer() {
        let file = parse_identity("00:1a:765").unwrap();
        let negative = format!("-{}", NodeProcess::MINIMUM);
        let foreign = format!(
            "1: FLOCK ADVISORY WRITE {negative} 00:1a:766 0 EOF\n\
             2: FLOCK ADVISORY WRITE not-a-pid 00:1a:767 0 EOF\n\
             3: FLOCK ADVISORY WRITE not-a-pid unknown:identity 0 EOF\n\
             4: FLOCK ADVISORY WRITE"
        );
        assert!(LockTable(foreign.clone()).writer(&file).unwrap().is_none());
        let matching = "5: FLOCK ADVISORY WRITE 8123 00:1a:765 0 EOF";
        for text in [
            format!("{foreign}\n{matching}"),
            format!("{matching}\n{foreign}"),
        ] {
            let writer = LockTable(text).writer(&file).unwrap().unwrap();
            assert_eq!(writer.process, NodeProcess(8123));
            assert_eq!(writer.file, file);
        }
        for process in [negative.as_str(), "not-a-pid"] {
            let table = LockTable(format!("5: FLOCK ADVISORY WRITE {process} 00:1a:765 0 EOF"));
            assert!(matches!(table.writer(&file), Err(DecodeError::Number(_))));
        }
    }
}
