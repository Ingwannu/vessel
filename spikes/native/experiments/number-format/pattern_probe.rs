//! Unlinked folder-pattern candidate. Fluent owns parsing and rendering;
//! this layer reads only the compact scale literals from its maintained AST.
//! Whole English fallback belongs to the caller, including its numeric body.

use std::{borrow::Cow, collections::BTreeSet, sync::Arc};

use fluent_bundle::{FluentArgs, FluentError, FluentResource, concurrent::FluentBundle};
use fluent_syntax::{ast, parser::ParserError};
use icu_plurals::PluralCategory;

use super::decimal_probe::{CompactExponent, CompactScales, ScaleError};

const QUANTITY: &str = "number-cost-quantity";
const USD: &str = "number-cost-usd";
const COMPACT: &str = "number-cost-compact";
const COMPACT_BODY: &str = "number-cost-compact-body";
const DEFAULT_EXPONENT: i16 = 0;

#[derive(Debug, Clone, thiserror::Error)]
pub(super) enum PatternError {
    #[error("invalid numeric pattern locale: {0}")]
    Locale(#[source] Arc<unic_langid::LanguageIdentifierError>),
    #[error("numeric pattern parse errors: {0:?}")]
    Parse(Vec<ParserError>),
    #[error("numeric pattern catalog errors: {0:?}")]
    Bundle(Vec<FluentError>),
    #[error("missing numeric pattern {0}")]
    Missing(&'static str),
    #[error("compact body needs an exponent selector and numeric scale keys")]
    Selector,
    #[error("compact scale {value} is invalid: {source}")]
    ScaleLiteral {
        value: String,
        source: std::num::ParseIntError,
    },
    #[error("compact scale {0} is duplicated")]
    DuplicateScale(i16),
    #[error("the default compact scale must be zero")]
    DefaultScale,
    #[error(transparent)]
    Scales(#[from] ScaleError),
    #[error("numeric pattern {key} failed: {errors:?}")]
    Resolve {
        key: &'static str,
        errors: Vec<FluentError>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NumberSign {
    Positive,
    Negative,
}

impl NumberSign {
    fn keyword(self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum NumberParts<'a> {
    Quantity {
        absolute: &'a str,
        sign: NumberSign,
    },
    Usd {
        absolute: &'a str,
        sign: NumberSign,
    },
    Compact {
        absolute: &'a str,
        sign: NumberSign,
        exponent: CompactExponent,
        category: PluralCategory,
    },
}

pub(super) struct Patterns {
    bundle: FluentBundle<FluentResource>,
    scales: Vec<i16>,
}

impl std::fmt::Debug for Patterns {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Patterns")
            .field("scales", &self.scales)
            .finish_non_exhaustive()
    }
}

impl Patterns {
    pub(super) fn new(tag: &str, source: &str) -> Result<Self, PatternError> {
        let language = tag
            .parse()
            .map_err(|error| PatternError::Locale(Arc::new(error)))?;
        let resource = FluentResource::try_new(source.to_owned())
            .map_err(|(_, errors)| PatternError::Parse(errors))?;
        let scales = extract_scales(&resource)?;
        let _validated = CompactScales::new(&scales)?;
        let mut bundle = FluentBundle::new_concurrent(vec![language]);
        bundle.set_use_isolating(false);
        bundle
            .add_resource(resource)
            .map_err(PatternError::Bundle)?;
        Ok(Self { bundle, scales })
    }

    pub(super) fn scales(&self) -> Result<CompactScales<'_>, ScaleError> {
        CompactScales::new(&self.scales)
    }

    pub(super) fn render(&self, parts: NumberParts<'_>) -> Result<String, PatternError> {
        let mut args = FluentArgs::new();
        let (key, absolute, sign) = match parts {
            NumberParts::Quantity { absolute, sign } => (QUANTITY, absolute, sign),
            NumberParts::Usd { absolute, sign } => (USD, absolute, sign),
            NumberParts::Compact {
                absolute,
                sign,
                exponent,
                category,
            } => {
                args.set("exponent", exponent.get());
                args.set("category", category_keyword(category));
                (COMPACT, absolute, sign)
            }
        };
        args.set("absolute", absolute);
        args.set("sign", sign.keyword());
        let pattern = self
            .bundle
            .get_message(key)
            .and_then(|message| message.value())
            .ok_or(PatternError::Missing(key))?;
        let mut errors = Vec::new();
        let value: Cow<'_, str> = self
            .bundle
            .format_pattern(pattern, Some(&args), &mut errors);
        if errors.is_empty() {
            Ok(value.into_owned())
        } else {
            Err(PatternError::Resolve { key, errors })
        }
    }
}

fn category_keyword(category: PluralCategory) -> &'static str {
    match category {
        PluralCategory::Zero => "zero",
        PluralCategory::One => "one",
        PluralCategory::Two => "two",
        PluralCategory::Few => "few",
        PluralCategory::Many => "many",
        PluralCategory::Other => "other",
    }
}

fn extract_scales(resource: &FluentResource) -> Result<Vec<i16>, PatternError> {
    let pattern = resource
        .entries()
        .find_map(|entry| match entry {
            ast::Entry::Message(message) if message.id.name == COMPACT_BODY => {
                message.value.as_ref()
            }
            _ => None,
        })
        .ok_or(PatternError::Missing(COMPACT_BODY))?;
    let variants = pattern
        .elements
        .iter()
        .find_map(|element| match element {
            ast::PatternElement::Placeable {
                expression:
                    ast::Expression::Select {
                        selector: ast::InlineExpression::VariableReference { id },
                        variants,
                    },
            } if id.name == "exponent" => Some(variants),
            _ => None,
        })
        .ok_or(PatternError::Selector)?;
    let mut scales = BTreeSet::new();
    let mut zero_default = false;
    for variant in variants {
        let ast::VariantKey::NumberLiteral { value } = variant.key else {
            return Err(PatternError::Selector);
        };
        let value = value
            .parse::<i16>()
            .map_err(|source| PatternError::ScaleLiteral {
                value: value.to_owned(),
                source,
            })?;
        if !scales.insert(value) {
            return Err(PatternError::DuplicateScale(value));
        }
        if variant.default {
            if value != DEFAULT_EXPONENT {
                return Err(PatternError::DefaultScale);
            }
            zero_default = true;
        }
    }
    if !zero_default {
        return Err(PatternError::DefaultScale);
    }
    let _default_scale = scales.remove(&DEFAULT_EXPONENT);
    Ok(scales.into_iter().collect())
}

#[cfg(test)]
#[path = "pattern_probe_tests.rs"]
mod tests;
