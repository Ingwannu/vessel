//! Prepare whole storage explanations from typed facts at state/language events.
//!
//! General's path, source position and known field stay data. The translation
//! owns the whole recovery sentence; Rust never joins translated clauses. Two
//! shapes (known field or closed problem), each with/without a source position,
//! avoid passing invented fields or positions to satisfy a message contract.

use crate::files::{FileFault, FileProblem};
use crate::localization::{
    FormatError, General, GeneralKeptUnreadableAtProblem, GeneralKeptUnreadableProblem, Number,
};
use crate::prefs::{Kept, LaunchOnly};

/// A storage explanation could not be prepared; diagnostics are not UI words.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// A message failed in both its language and English fallback.
    #[error(transparent)]
    Format(#[from] FormatError),
    /// A source position is outside the formatter's exact count range.
    #[error("file position exceeds the exact translation count range: {0}")]
    Position(#[from] std::num::TryFromIntError),
}

impl General<'_> {
    /// Prepare the whole General storage hint, outside a renderer.
    ///
    /// # Errors
    /// Returns a formatting failure or a position outside the exact count range.
    pub fn storage(self, kept: &Kept) -> Result<String, StorageError> {
        match kept {
            Kept::File { path } => Ok(self.kept_file(path)?),
            Kept::ThisLaunch { why } => match why {
                LaunchOnly::NoPlace => Ok(self.kept_no_place().to_owned()),
                // A path discovery diagnostic has no translated contract. It
                // must never be shown as if it were a localized explanation.
                LaunchOnly::NoPath { .. } => Ok(self.kept_no_path().to_owned()),
                LaunchOnly::Newer { path, version } => {
                    Ok(self.kept_newer(path, Number::count(*version))?)
                }
                LaunchOnly::Unreadable { fault } => unreadable(self, fault),
            },
        }
    }
}

fn unreadable(words: General<'_>, fault: &FileFault) -> Result<String, StorageError> {
    let position = fault
        .at
        .map(|(line, column)| {
            Ok::<_, StorageError>((
                Number::count(u32::try_from(line)?),
                Number::count(u32::try_from(column)?),
            ))
        })
        .transpose()?;
    if let (FileProblem::WrongKind, Some(field)) = (fault.problem, fault.field.as_deref())
        && field != "token"
    {
        return match position {
            Some((line, column)) => {
                Ok(words.kept_unreadable_field_at(column, field, line, &fault.path)?)
            }
            None => Ok(words.kept_unreadable_field(field, &fault.path)?),
        };
    }
    let problem = match fault.problem {
        FileProblem::NotToml => GeneralKeptUnreadableProblem::NotToml,
        FileProblem::NotJson => GeneralKeptUnreadableProblem::NotJson,
        FileProblem::UnknownKey => GeneralKeptUnreadableProblem::UnknownKey,
        FileProblem::DuplicateKey => GeneralKeptUnreadableProblem::DuplicateKey,
        FileProblem::MissingKey => GeneralKeptUnreadableProblem::MissingKey,
        FileProblem::WrongKind => {
            if fault.field.as_deref() == Some("token") {
                GeneralKeptUnreadableProblem::WrongToken
            } else {
                GeneralKeptUnreadableProblem::WrongValue
            }
        }
        FileProblem::CannotOpen => GeneralKeptUnreadableProblem::CannotOpen,
    };
    match position {
        Some((line, column)) => {
            Ok(words.kept_unreadable_at(column, line, &fault.path, problem.into())?)
        }
        None => Ok(words.kept_unreadable(&fault.path, problem)?),
    }
}

impl From<GeneralKeptUnreadableProblem> for GeneralKeptUnreadableAtProblem {
    fn from(problem: GeneralKeptUnreadableProblem) -> Self {
        match problem {
            GeneralKeptUnreadableProblem::NotToml => Self::NotToml,
            GeneralKeptUnreadableProblem::NotJson => Self::NotJson,
            GeneralKeptUnreadableProblem::UnknownKey => Self::UnknownKey,
            GeneralKeptUnreadableProblem::DuplicateKey => Self::DuplicateKey,
            GeneralKeptUnreadableProblem::MissingKey => Self::MissingKey,
            GeneralKeptUnreadableProblem::WrongToken => Self::WrongToken,
            GeneralKeptUnreadableProblem::WrongValue => Self::WrongValue,
            GeneralKeptUnreadableProblem::CannotOpen => Self::CannotOpen,
        }
    }
}

#[cfg(test)]
mod tests;
