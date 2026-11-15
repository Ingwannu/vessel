//! Fluent resolution and typed diagnostics, below bundle configuration and caches.
//! Resource ownership is generic so core resources and installation-owned Arcs
//! share the same resolver without loading plugin messages into core caches.
use fluent_bundle::{FluentArgs, FluentError, FluentResource, concurrent::FluentBundle};
use fluent_syntax::parser::ParserError;
use std::{
    borrow::{Borrow, Cow},
    sync::Arc,
};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Transform {
    Original,
    Pseudo,
}

pub(super) type Bundle<R = FluentResource> = FluentBundle<R>;

/// Embedded catalog initialization failed. These are diagnostics, never UI
/// copy; a host logs the cause before drawing its own localized error.
#[derive(Debug, Clone, Error)]
pub enum InitError {
    /// A locale absent from this build was requested.
    #[error("translation catalog index is absent")]
    MissingLocale,
    /// The generated numeric-rule contract is absent.
    #[error("English formatting rules are absent")]
    FormatRules,
    /// An embedded tag is invalid.
    #[error("invalid translation locale: {0}")]
    Locale(#[source] Arc<unic_langid::LanguageIdentifierError>),
    /// A resource did not parse; retain every parser diagnostic.
    #[error("invalid Fluent resource: {0:?}")]
    Parse(Vec<ParserError>),
    /// Resource identifiers or grammar functions collided.
    #[error("Fluent catalog errors: {0:?}")]
    Bundle(Vec<FluentError>),
    /// A plain value could not be resolved before drawing.
    #[error(transparent)]
    Plain(#[from] FormatError),
}

/// A generated dynamic message could not resolve in either language.
#[derive(Debug, Clone, Error)]
pub enum FormatError {
    /// Internal typed date encoding failed.
    #[error("cannot encode date parts: {0}")]
    DateEncoding(#[source] Arc<serde_json::Error>),
    /// English generation and its resources disagree.
    #[error("missing English message {0}")]
    Missing(&'static str),
    /// Retain both diagnostics when a translation and its fallback fail.
    #[error("translation failed ({translation}); English fallback failed: {english}")]
    Fallback {
        /// The translated message's diagnostic.
        translation: Box<Self>,
        /// The final English diagnostic.
        #[source]
        english: Box<Self>,
    },
    /// Retains Fluent's diagnostics without displaying them as app words.
    #[error("cannot format {key}: {errors:?}")]
    Resolve {
        /// English message identifier.
        key: &'static str,
        /// Every error reported by the English formatter.
        errors: Vec<FluentError>,
    },
}

pub(super) fn pseudo(text: &str) -> Cow<'_, str> {
    fluent_pseudo::transform(text, false, true)
}

pub(super) fn resolve<R: Borrow<FluentResource>>(
    bundle: &Bundle<R>,
    english: &Bundle<R>,
    key: &'static str,
    args: Option<&FluentArgs<'_>>,
) -> Result<String, FormatError> {
    match format_one(bundle, key, args) {
        Ok(value) => Ok(value),
        Err(translation) => {
            format_one(english, key, args).map_err(|english| FormatError::Fallback {
                translation: Box::new(translation),
                english: Box::new(english),
            })
        }
    }
}

pub(super) fn format_one<R: Borrow<FluentResource>>(
    bundle: &Bundle<R>,
    key: &'static str,
    args: Option<&FluentArgs<'_>>,
) -> Result<String, FormatError> {
    let pattern = bundle
        .get_message(key)
        .and_then(|message| message.value())
        .ok_or(FormatError::Missing(key))?;
    let mut errors = Vec::new();
    let value = bundle.format_pattern(pattern, args, &mut errors);
    if errors.is_empty() {
        Ok(value.into_owned())
    } else {
        Err(FormatError::Resolve { key, errors })
    }
}
