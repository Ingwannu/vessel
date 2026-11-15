//! Retain typed failures privately; public diagnostics expose categories, not input.
use std::{error::Error, fmt};

/// A metadata decoder failure whose source text is private evidence.
/// Display, Debug and the public source chain expose only category and location.
pub struct ManifestDecodeError {
    cause: serde_json::Error,
}
impl From<serde_json::Error> for ManifestDecodeError {
    fn from(cause: serde_json::Error) -> Self {
        Self { cause }
    }
}
impl fmt::Debug for ManifestDecodeError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_struct("ManifestDecodeError")
            .field("category", &self.cause.classify())
            .field("line", &self.cause.line())
            .field("column", &self.cause.column())
            .finish()
    }
}
impl fmt::Display for ManifestDecodeError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "skill metadata {:?} at {}:{}",
            self.cause.classify(),
            self.cause.line(),
            self.cause.column()
        )
    }
}
impl Error for ManifestDecodeError {}

/// A filesystem failure whose custom message or nested cause is private evidence.
/// Kind and OS code remain available without quoting an input path or message.
pub struct SkillIoError {
    cause: std::io::Error,
}
impl From<std::io::Error> for SkillIoError {
    fn from(cause: std::io::Error) -> Self {
        Self { cause }
    }
}
impl fmt::Debug for SkillIoError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_struct("SkillIoError")
            .field("kind", &self.cause.kind())
            .field("os_code", &self.cause.raw_os_error())
            .finish()
    }
}
impl fmt::Display for SkillIoError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "skill filesystem operation failed ({:?})",
            self.cause.kind()
        )
    }
}
impl Error for SkillIoError {}

/// The owner's typed selection I/O failure, retained without public source access.
/// Arbitrary owner errors may contain input in every formatting implementation.
pub struct SelectionOwnerError<E: Error + 'static> {
    _cause: E,
}
impl<E: Error + 'static> From<E> for SelectionOwnerError<E> {
    fn from(cause: E) -> Self {
        Self { _cause: cause }
    }
}
impl<E: Error + 'static> fmt::Debug for SelectionOwnerError<E> {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_struct("SelectionOwnerError")
            .field("owner", &std::any::type_name::<E>())
            .finish_non_exhaustive()
    }
}
impl<E: Error + 'static> fmt::Display for SelectionOwnerError<E> {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str("skill selection owner operation failed")
    }
}
impl<E: Error + 'static> Error for SelectionOwnerError<E> {}

#[cfg(test)]
mod tests;
