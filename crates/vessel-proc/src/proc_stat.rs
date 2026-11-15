//! Linux stat decoding inside the bounded process-observation edge.
//! Command-name bytes stay opaque; only the selected numeric fields escape.

/// Kernel start ticks have no wall-clock interpretation or PID authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StartTicks(pub(crate) u64);

/// Borrow one record during decoding, without exposing command text in Debug.
pub(crate) struct Stat<'a> {
    process: &'a [u8],
    rest: &'a [u8],
}

impl<'a> Stat<'a> {
    /// Kernel format positions, shared with recorded-layout regression fixtures.
    pub(crate) const FIRST_AFTER_COMM: usize = 3;
    /// Linux stat's start-time field, independent of the opaque comm length.
    pub(crate) const START_TICKS: usize = 22;

    /// Split before numeric validation so callers retain their PID-first rule.
    ///
    /// # Errors
    /// A missing stat prefix separator refuses the record.
    pub(crate) fn split(bytes: &'a [u8]) -> Result<Self, StatError> {
        let mut prefix = bytes.splitn(2, |byte| *byte == b' ');
        let process = prefix.next().ok_or(StatError::Fields)?;
        let rest = prefix.next().ok_or(StatError::Fields)?;
        Ok(Self { process, rest })
    }

    /// Let each observation apply its existing typed PID admission first.
    pub(crate) fn process(&self) -> &[u8] {
        self.process
    }

    /// Decode the start field without interpreting the command-name bytes.
    ///
    /// # Errors
    /// Missing structure, invalid encoding or an invalid number refuses.
    pub(crate) fn started(&self) -> Result<StartTicks, StatError> {
        if self.rest.first() != Some(&b'(') {
            return Err(StatError::Fields);
        }
        // comm may contain spaces and ')'. The final ')' precedes numeric
        // fields, which contain neither, so its byte length is irrelevant.
        let close = self
            .rest
            .iter()
            .rposition(|byte| *byte == b')')
            .ok_or(StatError::Fields)?;
        let fields = self
            .rest
            .get(close.saturating_add(1)..)
            .ok_or(StatError::Fields)?;
        if !fields.first().is_some_and(u8::is_ascii_whitespace) {
            return Err(StatError::Fields);
        }
        let ticks = fields
            .split(u8::is_ascii_whitespace)
            .filter(|field| !field.is_empty())
            .nth(Self::START_TICKS.saturating_sub(Self::FIRST_AFTER_COMM))
            .ok_or(StatError::Fields)?;
        let text = std::str::from_utf8(ticks).map_err(StatError::Encoding)?;
        text.parse().map(StartTicks).map_err(StatError::Number)
    }
}

/// Safe format failures preserve causes without carrying record bytes.
#[derive(Debug, thiserror::Error)]
pub(crate) enum StatError {
    /// Missing structure must not be interpreted as a process generation.
    #[error("process generation record lacks a required field")]
    Fields,
    /// Only numeric fields require Unicode, never the opaque command bytes.
    #[error("process observation number is not valid Unicode")]
    Encoding(#[source] std::str::Utf8Error),
    /// Retain the original integer parse cause for the observation owner.
    #[error("process observation number cannot be decoded")]
    Number(#[source] std::num::ParseIntError),
}
