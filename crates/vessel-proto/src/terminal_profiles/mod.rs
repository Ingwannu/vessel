//! Administrator-configured terminal launch templates (ADR 0091).
//! This module validates and substitutes literal argv; it performs no I/O,
//! program lookup, shell interpolation or process creation.

mod text;
mod values;

pub use text::{Argument, Icon, Id, Name, Program, Prompt, TextKind};
pub use values::{Profile, Profiles, Resolved};

/// The reference profile format's literal placeholder, replaced in every entry.
pub const PROMPT_SENTINEL: &str = "{{{prompt}}}";
/// Whole-package version; only the integrator assigns it after consumers land.
pub const CREATION_PROTOCOL: Option<crate::ProtocolVersion> = None;
/// Maximum configured profiles, keeping settings and picker work bounded.
pub const MAX_PROFILES: usize = 64;
/// Maximum argv entries in one profile or resolved launch.
pub const MAX_ARGUMENTS: usize = 256;
/// Maximum combined command and argument bytes in one launch.
pub const MAX_COMMAND_BYTES: usize = 64 * 1024;

/// Invalid template data is refused before it can be persisted or executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ProfileError {
    /// An identity, display name or program must contain non-whitespace text.
    #[error("terminal profile {0:?} cannot be empty")]
    Empty(TextKind),
    /// OS arguments cannot represent NUL; names/icons cannot redraw a picker.
    #[error("terminal profile {0:?} contains a forbidden character")]
    Character(TextKind),
    /// A typed field exceeds its one declared byte bound.
    #[error("terminal profile {kind:?} exceeds {max} bytes")]
    TooLong {
        /// Which validating type owns the bound.
        kind: TextKind,
        /// The bound, never the rejected content.
        max: usize,
    },
    /// The default shell is a target, never an administrator-defined profile.
    #[error("the blank terminal profile id is reserved")]
    ReservedId,
    /// Two rows cannot name different programs with the same identity.
    #[error("terminal profile ids must be unique")]
    DuplicateId,
    /// A bounded ordered profile list cannot grow indefinitely.
    #[error("at most {MAX_PROFILES} terminal profiles are allowed")]
    TooManyProfiles,
    /// Keep argv iteration and process launch bounded.
    #[error("at most {MAX_ARGUMENTS} terminal arguments are allowed")]
    TooManyArguments,
    /// Substitution can grow a small template into an excessive launch.
    #[error("terminal command and arguments exceed {MAX_COMMAND_BYTES} bytes")]
    CommandTooLarge,
}

#[cfg(test)]
mod tests;
