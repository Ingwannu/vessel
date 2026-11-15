//! Validated Fluent names and typed contracts; source text stops at this edge.
use std::{
    borrow::Borrow,
    collections::{BTreeMap, BTreeSet},
    fmt,
    ops::Deref,
    sync::Arc,
};
use thiserror::Error;

/// One Fluent identifier, not arbitrary UI text or an application identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Name(Arc<str>);
impl Name {
    pub(crate) fn parse(value: &str) -> Result<Self, ContractError> {
        let mut bytes = value.bytes();
        if !bytes.next().is_some_and(|b| b.is_ascii_alphabetic())
            || !bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
        {
            return Err(ContractError::Identifier(value.to_owned()));
        }
        Ok(Self(value.into()))
    }
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}
impl Deref for Name {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}
impl Borrow<str> for Name {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}
impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

/// Message and term identifiers occupy different Fluent namespaces.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Key {
    Message(Name),
    Term(Name),
}
impl Key {
    pub(crate) fn parse(value: &str) -> Result<Self, ContractError> {
        match value.strip_prefix('-') {
            Some(term) => Name::parse(term).map(Self::Term),
            None => Name::parse(value).map(Self::Message),
        }
    }
}
impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(name) => fmt::Display::fmt(name, f),
            Self::Term(name) => write!(f, "-{name}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Kind {
    Text,
    Number,
    Date,
    Choice(BTreeSet<Name>),
}
/// Fluent named arguments are source literals; they cannot supply a typed Date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalLiteral {
    Text,
    Number,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BindingRequirement {
    Number,
    CallerDate,
}

#[derive(Debug, Clone)]
pub(crate) struct Reference {
    pub(super) key: Key,
    pub(super) bound: BTreeMap<Name, LocalLiteral>,
}
#[derive(Debug, Clone, Default)]
pub(crate) struct Shape {
    pub(super) conflicts: BTreeSet<Name>,
    pub(super) variables: BTreeMap<Name, Kind>,
    pub(super) references: Vec<Reference>,
}
impl Shape {
    pub(super) fn merge(&mut self, name: Name, kind: Kind) {
        let current = self.variables.entry(name.clone()).or_insert(Kind::Text);
        match (current, kind) {
            (Kind::Text | Kind::Number | Kind::Date | Kind::Choice(_), Kind::Text)
            | (Kind::Number, Kind::Number)
            | (Kind::Date, Kind::Date) => {}
            (Kind::Choice(left), Kind::Choice(right)) => left.extend(right),
            (slot @ Kind::Text, value @ (Kind::Number | Kind::Date | Kind::Choice(_))) => {
                *slot = value
            }
            (Kind::Number, Kind::Date | Kind::Choice(_))
            | (Kind::Date, Kind::Number | Kind::Choice(_))
            | (Kind::Choice(_), Kind::Number | Kind::Date) => {
                let _new = self.conflicts.insert(name);
            }
        }
    }
}

/// A checked transitive contract. Only reference expansion constructs it, so a
/// caller cannot compare raw shapes and accidentally hide referenced variables.
#[derive(Debug, Clone)]
pub(crate) struct Contract {
    pub(super) variables: BTreeMap<Name, Kind>,
}
impl Contract {
    pub(crate) fn variables(&self) -> &BTreeMap<Name, Kind> {
        &self.variables
    }
}

const UNARY_ARGUMENTS: usize = 1;
const PARTICLE_ARGUMENTS: usize = 2;

#[derive(Debug, Clone, Copy)]
pub(crate) enum CallArity {
    Unary,
    Particle,
}
impl CallArity {
    pub(super) const fn count(self) -> usize {
        match self {
            Self::Unary => UNARY_ARGUMENTS,
            Self::Particle => PARTICLE_ARGUMENTS,
        }
    }
}
impl fmt::Display for CallArity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.count(), f)
    }
}

#[derive(Debug, Error)]
pub(crate) enum ContractError {
    #[error("invalid Fluent identifier {0}")]
    Identifier(String),
    #[error("message attributes are not supported; use a contextual message key")]
    MessageAttribute,
    #[error("term attributes are not supported; use a named term")]
    TermAttribute,
    #[error("formatting patterns cannot call DATETIME recursively")]
    RecursiveDate,
    #[error("unknown function {0}")]
    UnknownFunction(Name),
    #[error("{function} needs {arity} positional arguments and no named options")]
    CallArguments { function: Name, arity: CallArity },
    #[error("DATETIME requires a typed date variable")]
    DateArgument,
    #[error("minimumFractionDigits exceeds maximumFractionDigits")]
    FractionOrder,
    #[error(
        "unsupported NUMBER option; use grouping, integer/fraction digits (0–9), or plural type"
    )]
    NumberOption,
    #[error("PARTICLE's second argument must name object, subject, topic, with, or direction")]
    ParticleArgument,
    #[error("invalid numeric literal: {0}")]
    NumberParse(#[source] std::num::ParseFloatError),
    #[error("numeric literals require u32 magnitude and at most nine decimal places")]
    NumberRange,
    #[error("NUMBER requires a numeric literal, typed number variable, or NUMBER result")]
    NumberArgument,
    #[error("term arguments must be string or numeric literals")]
    TermArgument,
    #[error("{key} has an incompatible local value for ${name}: {required:?}")]
    BindingType {
        key: Key,
        name: Name,
        required: BindingRequirement,
    },
    #[error("cyclic or excessively deep reference at {0}")]
    ReferenceDepth(Key),
    #[error("cyclic or excessively deep reference at {0}")]
    ReferenceCycle(Key),
    #[error("missing reference {0}; translate it too or omit the referring message")]
    MissingReference(Key),
    #[error("{key} needs a locally bound ${name} parameter")]
    UnboundTerm { key: Key, name: Name },
    #[error("{key} uses incompatible types for ${variables}")]
    ConflictingTypes { key: Key, variables: Conflicts },
    #[error("{0} uses variables English does not pass")]
    ExtraVariables(Key),
    #[error("{0} must use the same variables as English")]
    MissingVariables(Key),
    #[error("{key}: ${name} requires a date but English passes another type")]
    DateRequired { key: Key, name: Name },
    #[error("{key}: ${name} requires a number but English passes text")]
    NumberRequired { key: Key, name: Name },
}
#[derive(Debug)]
pub(crate) struct Conflicts(pub(super) BTreeSet<Name>);
impl fmt::Display for Conflicts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, name) in self.0.iter().enumerate() {
            if index > 0 {
                write!(f, ", $")?;
            }
            fmt::Display::fmt(name, f)?;
        }
        Ok(())
    }
}
