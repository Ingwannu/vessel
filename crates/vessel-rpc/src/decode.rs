//! What a client frame that did not decode is reported as: where and what
//! kind of mistake, never the values it carried.
//!
//! [Decision Log]
//! - Purpose: the server answered a request whose parameters did not
//!   decode with `invalid request: {serde's message}`, and ended a
//!   connection on a frame that was not JSON with the serde error as the
//!   cause, which the node logs with its source chain. Serde's
//!   `invalid type` / `invalid value` messages quote the input (`invalid
//!   type: string "…", expected u32`), so any method's parameters (a
//!   secret, audio, settings) could be echoed to the caller and written to
//!   the node's log (dictation's source reading, 2026-11-15).
//! - Constraint: the answer must still say enough to fix the request, and
//!   every method shares this one decoder; per-type redaction in each
//!   request's `Deserialize` would be many edges to keep right.
//! - Alternatives: rewrite serde's message, as the profile parser's
//!   `without_values` does (it keeps "unknown field `…`" and validators'
//!   own words, both of which can carry input); keep the error but print
//!   a fixed text (the source chain still reaches the log).
//! - Choice: a typed [`DecodeFailure`] that keeps only the method (when it
//!   is a known one), the field path (segments that are protocol field
//!   names; anything else becomes a placeholder) and the error's kind. It
//!   has no source, and its Display and Debug are built from those parts
//!   alone. The path comes from `serde_path_to_error`, already in the
//!   workspace's lock.
//! - Cost: a validator's own reason ("label key too long") is reported as
//!   "an invalid value" at its path; the caller sees where, not why.

use std::fmt::{self, Write as _};

use serde_path_to_error::Segment;
use vessel_proto::Method;

/// At most this many path segments are kept; deeper ones are summarised.
const MAX_SEGMENTS: usize = 12;

/// The longest field name kept in a path; protocol names are far shorter.
const MAX_NAME: usize = 48;

/// Why a client frame did not decode, without any of its values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeFailure {
    method: Option<Method>,
    kind: DecodeKind,
    path: String,
}

/// What kind of mistake a frame had. Never carries input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeKind {
    /// Not valid JSON.
    Syntax,
    /// The frame ended early.
    Truncated,
    /// A required field is absent.
    MissingField,
    /// A field, method or variant name this build does not know.
    UnknownName,
    /// A value of the wrong JSON type.
    WrongType,
    /// A list or tuple of the wrong length.
    WrongLength,
    /// A value of the right type that the field does not accept.
    InvalidValue,
}

impl DecodeFailure {
    /// The failure of `error`, met at `path`, in a frame for `method` when
    /// that is a known method.
    pub(crate) fn new(
        method: Option<Method>,
        path: &serde_path_to_error::Path,
        error: &serde_json::Error,
    ) -> Self {
        Self {
            method,
            kind: DecodeKind::of(error),
            path: render(path),
        }
    }

    /// The kind of mistake.
    #[must_use]
    pub const fn kind(&self) -> DecodeKind {
        self.kind
    }

    /// The method the frame named, when it is a known one.
    #[must_use]
    pub const fn method(&self) -> Option<Method> {
        self.method
    }
}

impl DecodeKind {
    fn of(error: &serde_json::Error) -> Self {
        match error.classify() {
            serde_json::error::Category::Syntax | serde_json::error::Category::Io => Self::Syntax,
            serde_json::error::Category::Eof => Self::Truncated,
            serde_json::error::Category::Data => {
                // Only the start of serde's message is read, to pick the
                // kind; the message itself, which can quote the input, is
                // not kept.
                let message = error.to_string();
                if message.starts_with("missing field") {
                    Self::MissingField
                } else if message.starts_with("unknown field")
                    || message.starts_with("unknown variant")
                {
                    Self::UnknownName
                } else if message.starts_with("invalid type") {
                    Self::WrongType
                } else if message.starts_with("invalid length") {
                    Self::WrongLength
                } else {
                    Self::InvalidValue
                }
            }
        }
    }

    const fn words(self) -> &'static str {
        match self {
            Self::Syntax => "not valid JSON",
            Self::Truncated => "the frame ends early",
            Self::MissingField => "a required field is missing",
            Self::UnknownName => "a name this node does not know",
            Self::WrongType => "a value of the wrong type",
            Self::WrongLength => "a list of the wrong length",
            Self::InvalidValue => "a value the field does not accept",
        }
    }
}

/// The path with protocol field names kept and anything else (map keys,
/// unknown names chosen by the caller) replaced, bounded.
fn render(path: &serde_path_to_error::Path) -> String {
    let mut out = String::new();
    for (index, segment) in path.iter().enumerate() {
        if index == MAX_SEGMENTS {
            out.push_str(".…");
            break;
        }
        match segment {
            Segment::Seq { index } => {
                let _infallible = write!(out, "[{index}]");
            }
            Segment::Map { key } | Segment::Enum { variant: key } => {
                if !out.is_empty() {
                    out.push('.');
                }
                out.push_str(if is_field_name(key) { key } else { "<key>" });
            }
            Segment::Unknown => {
                if !out.is_empty() {
                    out.push('.');
                }
                out.push('?');
            }
        }
    }
    out
}

/// A protocol field name: lower-case letters, digits and underscores,
/// starting with a letter, short.
fn is_field_name(key: &str) -> bool {
    key.len() <= MAX_NAME
        && key.starts_with(|c: char| c.is_ascii_lowercase())
        && key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

impl fmt::Display for DecodeFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(method) = self.method {
            write!(f, "`{}`: ", method.as_str())?;
        }
        f.write_str(self.kind.words())?;
        if !self.path.is_empty() {
            write!(f, " at `{}`", self.path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
