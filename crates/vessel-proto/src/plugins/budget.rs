//! Count encoded JSON without allocating a second unbounded serialized copy.
use super::PluginJsonError;
use serde::Serialize;
use std::io;

pub(super) enum EncodingError {
    Exceeded,
    Encode(PluginJsonError),
}

pub(super) fn encoded_bytes<T: Serialize>(value: &T, limit: usize) -> Result<usize, EncodingError> {
    let mut sink = ByteBudget {
        used: 0,
        limit,
        exceeded: false,
    };
    serde_json::to_writer(&mut sink, value).map_err(|cause| {
        if sink.exceeded {
            EncodingError::Exceeded
        } else {
            EncodingError::Encode(PluginJsonError::new(cause))
        }
    })?;
    Ok(sink.used)
}

struct ByteBudget {
    used: usize,
    limit: usize,
    exceeded: bool,
}
impl io::Write for ByteBudget {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let Some(total) = self
            .used
            .checked_add(bytes.len())
            .filter(|total| *total <= self.limit)
        else {
            self.exceeded = true;
            return Err(io::ErrorKind::WriteZero.into());
        };
        self.used = total;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
