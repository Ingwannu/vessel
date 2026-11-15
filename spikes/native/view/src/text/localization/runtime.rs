//! Prepared core catalogs: resources and plain values are initialized once.
//! Plugin resources use the shared configuration factory but never enter these
//! process-wide caches or the English-generated core API.
use super::resolution::{Bundle, FormatError, InitError, resolve};
use super::{CACHES, EMBEDDED, ENGLISH, Locale, PSEUDO, Plain, resources};
use fluent_bundle::{FluentArgs, FluentResource};

#[derive(Debug)]
pub(super) struct Embedded {
    pub(super) sources: &'static [&'static str],
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
        let resources::Bundles {
            mut localized,
            mut fallback,
        } = resources::bundles(locale)?;
        add_sources(&mut localized, embedded.sources)?;
        add_sources(&mut fallback, english.sources)?;
        let (translated, english) = (localized, fallback);
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

fn add_sources(bundle: &mut Bundle, sources: &[&str]) -> Result<(), InitError> {
    for source in sources {
        let resource = FluentResource::try_new((*source).to_owned())
            .map_err(|(_, errors)| InitError::Parse(errors))?;
        bundle.add_resource(resource).map_err(InitError::Bundle)?;
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn fixture(tag: &str, source: &str) -> Result<Bundle, InitError> {
    let mut bundle = resources::configured(tag, &[source], super::resolution::Transform::Original)?;
    add_sources(&mut bundle, &[source])?;
    Ok(bundle)
}
