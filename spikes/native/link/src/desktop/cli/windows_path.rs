//! Preserve raw UTF-16 PATH entries; never route a Unicode path through cmd.exe.

use super::super::DesktopError;
use vessel_app_view::desktop::cli::Operation;

#[cfg(windows)]
mod registry;
#[cfg(windows)]
pub(super) use registry::{apply, status, validate};

const MAX_PATH_UNITS: usize = 32_767;
const SEPARATOR: u16 = 59;
const PERCENT: u16 = 37;

/// One literal directory, rather than an expandable expression or a PATH list.
struct Entry(Vec<u16>);

impl Entry {
    fn new(units: Vec<u16>) -> Result<Self, DesktopError> {
        if units.is_empty()
            || units.len() >= MAX_PATH_UNITS
            || units
                .iter()
                .any(|unit| matches!(*unit, 0 | SEPARATOR | PERCENT))
        {
            return Err(DesktopError::InvalidUserPath);
        }
        Ok(Self(units))
    }
}

/// The registry terminator is validated once, preserving unpaired UTF-16 units.
struct UserPath(Vec<u16>);

impl UserPath {
    fn decode(bytes: &[u8]) -> Result<Self, DesktopError> {
        if bytes.len() % 2 != 0 || bytes.len() / 2 > MAX_PATH_UNITS {
            return Err(DesktopError::InvalidUserPath);
        }
        let mut units: Vec<u16> = bytes
            .chunks_exact(2)
            .filter_map(|pair| pair.first().zip(pair.get(1)))
            .map(|(low, high)| u16::from_le_bytes([*low, *high]))
            .collect();
        if units.pop() != Some(0) || units.contains(&0) {
            return Err(DesktopError::InvalidUserPath);
        }
        Ok(Self(units))
    }

    fn contains(&self, entry: &Entry) -> bool {
        self.0
            .split(|unit| *unit == SEPARATOR)
            .any(|part| same_entry(part, &entry.0))
    }

    fn changed(&self, entry: &Entry, operation: Operation) -> Result<Self, DesktopError> {
        let mut units = self.0.clone();
        match operation {
            Operation::Install if !self.contains(entry) => {
                if !units.is_empty() {
                    units.push(SEPARATOR);
                }
                units.extend_from_slice(&entry.0);
            }
            Operation::Install => {}
            Operation::Remove => {
                units = self
                    .0
                    .split(|unit| *unit == SEPARATOR)
                    .filter(|part| !same_entry(part, &entry.0))
                    .collect::<Vec<_>>()
                    .join(&SEPARATOR);
            }
        }
        if units.len() >= MAX_PATH_UNITS {
            return Err(DesktopError::InvalidUserPath);
        }
        Ok(Self(units))
    }

    fn encode(&self) -> Vec<u8> {
        self.0
            .iter()
            .copied()
            .chain(std::iter::once(0))
            .flat_map(u16::to_le_bytes)
            .collect()
    }
}

// Fold one Unicode scalar at a time, preserving unpaired Windows path units.
// Expanding mappings (for example ß -> SS) are not binary path equality.
fn same_entry(left: &[u16], right: &[u16]) -> bool {
    fn folded(unit: Result<char, std::char::DecodeUtf16Error>) -> Result<char, u16> {
        unit.map(|character| {
            let mut upper = character.to_uppercase();
            match (upper.next(), upper.next()) {
                (Some(single), None) => single,
                (Some(_), Some(_)) | (None, Some(_) | None) => character,
            }
        })
        .map_err(|invalid| invalid.unpaired_surrogate())
    }
    char::decode_utf16(left.iter().copied())
        .map(folded)
        .eq(char::decode_utf16(right.iter().copied()).map(folded))
}

#[cfg(test)]
mod tests;
