//! Build-time localization tooling, also compiled into gate tests and the
//! coverage example. Only this layer reads files; none of it ships in view.

mod catalog;
mod format_rules;
mod generate;
mod pattern;
mod validate;

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

pub(crate) use catalog::read;
pub(crate) use format_rules::generate as generate_format_rules;
pub(crate) use generate::generate;
pub(crate) use validate::validate;

/// A failure in a translation source or its generated API.
#[derive(Debug, Error)]
pub(crate) enum BuildError {
    #[error("translation tooling I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("{file}: {detail}")]
    Source { file: String, detail: String },
    #[error("cannot generate translation API: {0}")]
    Write(#[from] std::fmt::Error),
    #[error("translation build environment: {0}")]
    Environment(#[from] std::env::VarError),
}

fn source(file: impl Into<String>, detail: impl Into<String>) -> BuildError {
    BuildError::Source {
        file: file.into(),
        detail: detail.into(),
    }
}

/// An English argument's generated type; translations never define the API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Kind {
    Text,
    Number,
    Date,
    Choice(BTreeSet<String>),
}

/// A reference may bind a term's parameters locally.
#[derive(Debug, Clone)]
pub(crate) struct Reference {
    pub(crate) key: String,
    pub(crate) bound: BTreeSet<String>,
}

/// The transitive variable contract of one message or term.
#[derive(Debug, Clone, Default)]
pub(crate) struct Shape {
    pub(crate) conflicts: BTreeSet<String>,
    pub(crate) variables: BTreeMap<String, Kind>,
    pub(crate) references: Vec<Reference>,
}

/// One parsed, owned message; kept small so tests can construct bad catalogs.
#[derive(Debug, Clone)]
pub(crate) struct Message {
    pub(crate) area: String,
    pub(crate) context: String,
    pub(crate) shape: Shape,
    pub(crate) plain: Option<String>,
}

/// One folder's resources. Keys are global Fluent identifiers.
#[derive(Debug, Clone)]
pub(crate) struct Language {
    pub(crate) tag: String,
    pub(crate) files: BTreeMap<String, String>,
    pub(crate) messages: BTreeMap<String, Message>,
    pub(crate) terms: BTreeMap<String, Shape>,
}

/// Missing translations are data a contributor can inspect, not a build failure.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Coverage {
    pub(crate) tag: String,
    pub(crate) missing: Vec<String>,
}

#[cfg(test)]
mod tests;
