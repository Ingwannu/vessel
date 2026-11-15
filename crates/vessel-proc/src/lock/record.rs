//! Bounded bytes inside the proc-observation decoding edge.
//! No buffer contents belong in a diagnostic or a caller's domain value.

use std::io::{self, Read};

/// Keep byte budgeting inside the external-format decoder.
pub(super) struct Record(Vec<u8>);

impl Record {
    /// One read cap bounds allocation before inspecting any record field.
    pub(super) const MAX_BYTES: u64 = 65_536;

    /// Read only this observation record, retaining the original I/O cause.
    ///
    /// # Errors
    /// The record is inaccessible or exceeds the named byte cap.
    pub(super) fn read(path: &std::path::Path) -> io::Result<Self> {
        Self::from_reader(std::fs::File::open(path)?)
    }

    fn from_reader(reader: impl Read) -> io::Result<Self> {
        let mut bytes = Vec::new();
        // The extra byte detects excess; saturating arithmetic cannot wrap
        // the limit into a smaller read.
        let count = reader
            .take(Self::MAX_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)?;
        let maximum = usize::try_from(Self::MAX_BYTES)
            .map_err(ReadError::Length)
            .map_err(io::Error::other)?;
        if count > maximum {
            return Err(io::Error::other(ReadError::TooLong));
        }
        Ok(Self(bytes))
    }

    /// Borrow for one format decode; never pass these bytes into domain state.
    pub(super) fn bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, thiserror::Error)]
enum ReadError {
    #[error("process observation record exceeds {} bytes", Record::MAX_BYTES)]
    TooLong,
    #[error("process observation record length cannot be represented")]
    Length(#[source] std::num::TryFromIntError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_byte_cap_is_checked_before_any_format_decoding() {
        assert!(Record::from_reader(io::repeat(b'x').take(Record::MAX_BYTES)).is_ok());
        let Err(error) = Record::from_reader(io::repeat(b'x')) else {
            panic!("accepted unlimited input");
        };
        assert!(matches!(
            error.get_ref().and_then(|e| e.downcast_ref::<ReadError>()),
            Some(ReadError::TooLong)
        ));
        assert!(error.to_string().contains(&Record::MAX_BYTES.to_string()));
    }
}
