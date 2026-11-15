//! External parser/contract diagnostics must not repeat plugin-authored values.

use std::fmt;

use fluent_bundle::FluentError;
use fluent_syntax::parser::ParserError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Budget {
    Locales,
    Source,
    Total,
    Keys,
    Name,
    Arguments,
    Syntax,
    Output,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum Problem {
    #[error("plugin translation resource budget exceeded ({0:?})")]
    Budget(Budget),
    // unic-langid errors are closed enum classifications, never input strings.
    #[error("invalid plugin translation locale")]
    Locale(#[source] unic_langid::LanguageIdentifierError),
    #[error("plugin translations require English")]
    MissingEnglish,
    #[error("plugin translations contain a duplicate identity")]
    Duplicate,
    #[error("plugin translation attributes are unsupported")]
    Attributes,
    #[error("plugin translations contain an unsupported entry")]
    Entry,
    #[error("plugin translations contain an absent or empty message")]
    MissingMessage,
    #[error("plugin translations have invalid syntax")]
    Syntax(#[source] SyntaxFailure),
    #[error("plugin translation contract is invalid")]
    Contract(#[source] Sealed<crate::fluent_contract::ContractError>),
    #[error("plugin translation arguments do not match English")]
    Arguments,
    #[error("plugin translation bundle rejected its resources")]
    Bundle(#[source] BundleFailure),
    #[error("plugin translation formatter could not initialize")]
    Factory(#[source] Sealed<crate::localization::InitError>),
    #[error("plugin date argument could not be encoded")]
    Date(#[source] Sealed<crate::localization::FormatError>),
    #[error("plugin translation exceeds the target text contract")]
    Output(#[source] vessel_proto::plugins::InvalidPluginValue),
    #[error("plugin translation and English fallback both failed")]
    Fallback {
        translation: Box<Self>,
        #[source]
        english: Box<Self>,
    },
}

pub(super) struct Sealed<E>(E);

impl<E> Sealed<E> {
    pub(super) const fn new(cause: E) -> Self {
        Self(cause)
    }
}

impl<E: std::error::Error> fmt::Debug for Sealed<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sealed")
            .field("has_source", &self.0.source().is_some())
            .finish_non_exhaustive()
    }
}
impl<E> fmt::Display for Sealed<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid plugin resource contract")
    }
}
impl<E: std::error::Error> std::error::Error for Sealed<E> {}

pub(super) struct SyntaxFailure(pub(super) Vec<ParserError>);
impl fmt::Debug for SyntaxFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl fmt::Display for SyntaxFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} plugin syntax errors", self.0.len())
    }
}
impl std::error::Error for SyntaxFailure {}

pub(super) struct BundleFailure(pub(super) Vec<FluentError>);
impl fmt::Debug for BundleFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl fmt::Display for BundleFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} plugin formatting errors", self.0.len())
    }
}
impl std::error::Error for BundleFailure {}
