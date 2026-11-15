//! A terminal's title, command and file position all affect approved behaviour.
//! Compare by position, rather than merging duplicate titles or shell commands.
use super::command_changes::Change;
use vessel_proto::workspace::FileTerminal;

/// One zero-based position in the approved file's terminal list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalPosition(usize);

impl TerminalPosition {
    /// One-based ordinal for display; it is never a resource identity.
    #[must_use]
    pub const fn ordinal(self) -> usize {
        self.0.saturating_add(1)
    }
}

/// Complete current and previous terminal definitions, including name-only edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalLine {
    /// Kept, inserted or removed at this file position.
    Line {
        /// File order, independently of the optional title.
        position: TerminalPosition,
        /// Difference from the approved set.
        change: Change,
        /// Every field of the terminal definition.
        terminal: FileTerminal,
    },
    /// Changed at this position, retaining both complete definitions.
    Changed {
        /// Position whose behaviour changed.
        position: TerminalPosition,
        /// New definition.
        terminal: FileTerminal,
        /// Last approved definition.
        was: FileTerminal,
    },
}

impl TerminalLine {
    /// The current definition, or the removed definition when no current one exists.
    #[must_use]
    pub const fn terminal(&self) -> &FileTerminal {
        match self {
            Self::Line {
                position: _,
                change: _,
                terminal,
            }
            | Self::Changed {
                position: _,
                terminal,
                was: _,
            } => terminal,
        }
    }

    /// File order; duplicate names never collapse into a single row.
    #[must_use]
    pub const fn position(&self) -> TerminalPosition {
        match self {
            Self::Line {
                position,
                change: _,
                terminal: _,
            }
            | Self::Changed {
                position,
                terminal: _,
                was: _,
            } => *position,
        }
    }

    /// Whether comparison text must mention additional executable changes.
    #[must_use]
    pub const fn changed(&self) -> bool {
        !matches!(
            self,
            Self::Line {
                change: Change::Same,
                ..
            }
        )
    }
}

/// Positional comparison makes swaps, removals and duplicate titles explicit.
#[must_use]
pub fn compare(before: &[FileTerminal], after: &[FileTerminal]) -> Vec<TerminalLine> {
    (0..before.len().max(after.len()))
        .filter_map(|index| {
            let position = TerminalPosition(index);
            match (before.get(index), after.get(index)) {
                (Some(was), Some(terminal)) if was != terminal => Some(TerminalLine::Changed {
                    position,
                    terminal: terminal.clone(),
                    was: was.clone(),
                }),
                (_, Some(terminal)) => Some(TerminalLine::Line {
                    position,
                    change: if before.get(index).is_some() {
                        Change::Same
                    } else {
                        Change::Added
                    },
                    terminal: terminal.clone(),
                }),
                (Some(terminal), None) => Some(TerminalLine::Line {
                    position,
                    change: Change::Removed,
                    terminal: terminal.clone(),
                }),
                (None, None) => None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
