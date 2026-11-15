//! [Decision Log]
//! - Purpose: core and plugin-owned resources must share the same NUMBER,
//!   DATETIME, PARTICLE and pseudo behavior without sharing message namespaces.
//! - Choice: configure empty, resource-generic concurrent bundles from a
//!   validated embedded Locale. The caller owns resource addition and lifetime.
//! - Alternatives: a second plugin formatter drifts from the core contract;
//!   putting plugin resources in Translator leaks installations into its cache.
//! - Effect: date/month/weekday patterns come only from embedded core resources;
//!   plugin data cannot redefine the formatter's private pattern bundle.
use super::resolution::{Bundle, InitError, Transform, pseudo};
use super::{EMBEDDED, ENGLISH, Locale, PSEUDO, formats, grammar, number};
use fluent_bundle::FluentResource;
use std::borrow::Borrow;

/// Empty bundles with independent locale/plural rules. Callers add only their
/// own validated resources and prepare/cache the resulting messages on events.
pub(crate) struct Bundles<R> {
    pub(crate) localized: Bundle<R>,
    pub(crate) fallback: Bundle<R>,
}

impl<R> std::fmt::Debug for Bundles<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bundles").finish_non_exhaustive()
    }
}

/// Configure language and English bundles without importing core message keys.
/// The pseudo locale transforms both so missing plugin translations stay visible.
pub(crate) fn bundles<R: Borrow<FluentResource>>(locale: Locale) -> Result<Bundles<R>, InitError> {
    let source = if locale == PSEUDO { ENGLISH } else { locale };
    let transform = if locale == PSEUDO {
        Transform::Pseudo
    } else {
        Transform::Original
    };
    let sources = EMBEDDED
        .get(source.index)
        .ok_or(InitError::MissingLocale)?
        .sources;
    let english = EMBEDDED
        .get(ENGLISH.index)
        .ok_or(InitError::MissingLocale)?
        .sources;
    Ok(Bundles {
        localized: configured(source.tag(), sources, transform)?,
        fallback: configured(ENGLISH.tag(), english, transform)?,
    })
}

pub(super) fn configured<R: Borrow<FluentResource>>(
    tag: &str,
    sources: &[&str],
    transform: Transform,
) -> Result<Bundle<R>, InitError> {
    let language = tag
        .parse()
        .map_err(|error| InitError::Locale(std::sync::Arc::new(error)))?;
    let mut bundle = Bundle::new_concurrent(vec![language]);
    bundle
        .add_function("NUMBER", number::fluent)
        .map_err(|error| InitError::Bundle(vec![error]))?;
    bundle
        .add_function("PARTICLE", grammar::fluent)
        .map_err(|error| InitError::Bundle(vec![error]))?;
    formats::install(&mut bundle, tag, sources, transform)?;
    if transform == Transform::Pseudo {
        bundle.set_transform(Some(pseudo));
    }
    Ok(bundle)
}

#[cfg(test)]
mod tests;
