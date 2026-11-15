//! Navigation payload belongs to the file owner, not Target identity.
//! Reusing the protocol's one-based Unicode scalar position avoids byte,
//! UTF-16 and cursor offsets being confused at assistant/LSP boundaries.

use std::num::NonZeroU32;
use vessel_proto::lsp::{Position, Range};

/// A validated optional reveal request; it never changes root/path identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileLocation(Range);

/// Inverted source ranges are refused at the link or user-input boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a file location must end at or after its start")]
pub struct InvalidFileLocation;

impl FileLocation {
    /// A caret at one saved source position.
    #[must_use]
    pub const fn point(position: Position) -> Self {
        Self(Range {
            start: position,
            end: position,
        })
    }

    /// The first scalar column of a one-based line.
    #[must_use]
    pub const fn line(line: NonZeroU32) -> Self {
        Self::point(Position {
            line,
            column: NonZeroU32::MIN,
        })
    }

    /// Reveal complete lines, clamped to actual document length by the editor.
    /// The last column is deliberately maximal to include the final line's
    /// contents without guessing its UTF-8 byte or Unicode scalar length.
    ///
    /// # Errors
    /// [`InvalidFileLocation`] for an inverted inclusive line range.
    pub fn lines(start: NonZeroU32, end: NonZeroU32) -> Result<Self, InvalidFileLocation> {
        if end < start {
            return Err(InvalidFileLocation);
        }
        Self::try_from(Range {
            start: Position {
                line: start,
                column: NonZeroU32::MIN,
            },
            end: Position {
                line: end,
                column: NonZeroU32::MAX,
            },
        })
    }

    /// Canonical scalar coordinates for a renderer adapter.
    #[must_use]
    pub const fn range(self) -> Range {
        self.0
    }

    /// Only explicit ranges select text; a caret reveal does not select a line.
    #[must_use]
    pub fn selects(self) -> bool {
        self.0.start != self.0.end
    }
}

impl TryFrom<Range> for FileLocation {
    type Error = InvalidFileLocation;
    fn try_from(range: Range) -> Result<Self, Self::Error> {
        if range.end < range.start {
            Err(InvalidFileLocation)
        } else {
            Ok(Self(range))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lines_are_inclusive_and_ranges_cannot_invert_or_change_units() {
        let one = NonZeroU32::MIN;
        let two = NonZeroU32::new(2).unwrap();
        assert!(FileLocation::lines(two, one).is_err());
        let lines = FileLocation::lines(one, two).unwrap();
        assert_eq!(lines.range().start.column, one);
        assert_eq!(lines.range().end.line, two);
        assert_eq!(lines.range().end.column, NonZeroU32::MAX);
        assert!(lines.selects());
        assert!(!FileLocation::line(two).selects());
        let start = Position {
            line: two,
            column: two,
        };
        let end = Position {
            line: two,
            column: one,
        };
        assert!(FileLocation::try_from(Range { start, end }).is_err());
        assert_eq!(FileLocation::point(start).range().start, start);
    }
}
