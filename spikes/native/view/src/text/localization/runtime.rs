//! Initialization and resolution, separate from the generated field getters.
//! Each bundle keeps its own locale: English fallback therefore uses English
//! plural rules even when a partially translated locale has different rules.

use std::{borrow::Cow, sync::Arc};

use fluent_bundle::{FluentArgs, FluentError, FluentResource, concurrent::FluentBundle};
use fluent_syntax::parser::ParserError;
use thiserror::Error;

use super::{CACHES, EMBEDDED, ENGLISH, Locale, PSEUDO, Plain, grammar};

type BundleResult = Result<Bundle, InitError>;
pub(super) type Bundle = FluentBundle<FluentResource>;

#[derive(Debug)]
pub(super) struct Embedded {
    pub(super) sources: &'static [&'static str],
}

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

/// A prepared language: bundles and all resolved plain messages.
pub struct Translator {
    locale: Locale,
    bundle: Bundle,
    english: Bundle,
    pub(super) plain: Plain,
}

// Fluent bundles do not implement Debug. A locale identifies this cache
// without dumping every resource and the formatter's closure internals.
impl std::fmt::Debug for Translator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Translator")
            .field("locale", &self.locale)
            .finish_non_exhaustive()
    }
}

impl Translator {
    /// Prepare this language once, before drawing any screen. Subsequent
    /// calls borrow the process cache without parsing or allocating.
    ///
    /// # Errors
    /// Returns the retained initialization error for invalid embedded data.
    pub fn get(locale: Locale) -> Result<&'static Self, InitError> {
        CACHES
            .get(locale.index)
            .ok_or(InitError::MissingLocale)?
            .get_or_init(|| Self::new(locale))
            .as_ref()
            .map_err(Clone::clone)
    }

    /// The language of this prepared catalog.
    #[must_use]
    pub const fn locale(&self) -> Locale {
        self.locale
    }

    fn new(locale: Locale) -> Result<Self, InitError> {
        let source = if locale == PSEUDO { ENGLISH } else { locale };
        let embedded = EMBEDDED.get(source.index).ok_or(InitError::MissingLocale)?;
        let english = EMBEDDED
            .get(ENGLISH.index)
            .ok_or(InitError::MissingLocale)?;
        let translated = bundle(source.tag, embedded.sources, locale == PSEUDO)?;
        let english = bundle("en", english.sources, locale == PSEUDO)?;
        let plain = Plain::resolve(&translated, &english)?;
        Ok(Self {
            locale,
            bundle: translated,
            english,
            plain,
        })
    }

    pub(super) fn format(
        &self,
        key: &'static str,
        args: &FluentArgs<'_>,
    ) -> Result<String, FormatError> {
        resolve(&self.bundle, &self.english, key, Some(args))
    }
}

pub(super) fn pseudo(text: &str) -> Cow<'_, str> {
    fluent_pseudo::transform(text, false, true)
}

fn bundle(tag: &str, sources: &[&str], pseudo_locale: bool) -> BundleResult {
    let language = tag
        .parse::<unic_langid::LanguageIdentifier>()
        .map_err(|error| InitError::Locale(Arc::new(error)))?;
    let mut bundle = Bundle::new_concurrent(vec![language]);
    bundle
        .add_function("NUMBER", super::number::fluent)
        .map_err(|error| InitError::Bundle(vec![error]))?;
    bundle
        .add_function("PARTICLE", grammar::fluent)
        .map_err(|error| InitError::Bundle(vec![error]))?;
    super::formats::install(&mut bundle, tag, sources, pseudo_locale)?;
    if pseudo_locale {
        bundle.set_transform(Some(pseudo));
    }
    for source in sources {
        let resource = FluentResource::try_new((*source).to_owned())
            .map_err(|(_, errors)| InitError::Parse(errors))?;
        bundle.add_resource(resource).map_err(InitError::Bundle)?;
    }
    Ok(bundle)
}

pub(super) fn resolve(
    bundle: &Bundle,
    english: &Bundle,
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

pub(super) fn format_one(
    bundle: &Bundle,
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

#[cfg(test)]
pub(super) fn fixture(tag: &str, source: &str) -> Result<Bundle, InitError> {
    bundle(tag, &[source], false)
}
