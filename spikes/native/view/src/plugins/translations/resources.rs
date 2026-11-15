//! Validate whole namespaces before constructing event-time formatters.

use std::{collections::BTreeMap, sync::Arc};

use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use fluent_syntax::ast::Entry;

use crate::fluent_contract::{
    Contract, Key, Kind, PatternRole, Shape, VariableCoverage, compare, expanded, scan,
};
use crate::localization::{
    Locale,
    resources::{Bundles, bundles},
};
use vessel_proto::plugins::{Caption, ContentText, ThemeName};

use super::errors::{BundleFailure, Sealed, SyntaxFailure};
use super::{
    Argument, Arguments, Budget, LocaleTag, MAX_ARGUMENTS, MAX_FORMATTED_BYTES, MAX_KEYS,
    MAX_LOCALES, MAX_NAME_BYTES, MAX_RESOURCE_BYTES, MessageKey, Problem, Resource, checked_name,
};

struct Parsed {
    resource: Arc<FluentResource>,
    contracts: Arc<BTreeMap<Key, Contract>>,
}

/// One installation's namespace. No keys are inserted into core Translator or
/// another installation, even when local plugin/message names are identical.
pub(super) struct Resources {
    english: Parsed,
    locales: BTreeMap<LocaleTag, Parsed>,
}

impl std::fmt::Debug for Resources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resources")
            .field("locales", &self.locales.len())
            .finish_non_exhaustive()
    }
}

impl Resources {
    pub(super) fn new(sources: Vec<Resource>) -> Result<Self, Problem> {
        if sources.len() > MAX_LOCALES {
            return Err(Problem::Budget(Budget::Locales));
        }
        let mut locales = BTreeMap::new();
        let mut bytes = 0_usize;
        let mut keys = 0_usize;
        for source in sources {
            bytes = bytes.saturating_add(source.source.len());
            if bytes > MAX_RESOURCE_BYTES {
                return Err(Problem::Budget(Budget::Total));
            }
            let parsed = parse(&source)?;
            keys = keys.saturating_add(parsed.contracts.len());
            if keys > MAX_KEYS {
                return Err(Problem::Budget(Budget::Keys));
            }
            if locales.insert(source.locale, parsed).is_some() {
                return Err(Problem::Duplicate);
            }
        }
        let english = locales
            .remove(&LocaleTag(String::from("en")))
            .ok_or(Problem::MissingEnglish)?;
        for localized in locales.values() {
            validate_translation(&english, localized)?;
        }
        Ok(Self { english, locales })
    }

    /// Call once on catalog or language events. Missing locale/key formatting
    /// uses a separate English plural locale, never English text in a KO bundle.
    pub(super) fn prepare(&self, locale: Locale) -> Result<Formatter, Problem> {
        let mut prepared = bundles::<Arc<FluentResource>>(locale)
            .map_err(|cause| Problem::Factory(Sealed::new(cause)))?;
        prepared
            .fallback
            .add_resource(Arc::clone(&self.english.resource))
            .map_err(|errors| Problem::Bundle(BundleFailure(errors)))?;
        let translated = if locale.tag() == "en" {
            Some(&self.english)
        } else {
            self.locales.get(&LocaleTag(locale.tag().to_owned()))
        };
        if let Some(translated) = translated {
            prepared
                .localized
                .add_resource(Arc::clone(&translated.resource))
                .map_err(|errors| Problem::Bundle(BundleFailure(errors)))?;
        }
        Ok(Formatter {
            bundles: prepared,
            english: Arc::clone(&self.english.contracts),
        })
    }
}

fn parse(source: &Resource) -> Result<Parsed, Problem> {
    let resource = FluentResource::try_new(source.source.to_string())
        .map_err(|(_partial, errors)| Problem::Syntax(SyntaxFailure(errors)))?;
    let shapes = collect_shapes(&resource)?;
    let contracts = checked_contracts(&shapes)?;
    Ok(Parsed {
        resource: Arc::new(resource),
        contracts: Arc::new(contracts),
    })
}

fn collect_shapes(resource: &FluentResource) -> Result<BTreeMap<Key, Shape>, Problem> {
    let mut shapes = BTreeMap::new();
    for entry in resource.entries() {
        let (key, pattern) = match entry {
            Entry::Message(message) => {
                if !message.attributes.is_empty() {
                    return Err(Problem::Attributes);
                }
                (
                    Key::Message(checked_name(message.id.name)?),
                    message.value.as_ref().ok_or(Problem::MissingMessage)?,
                )
            }
            Entry::Term(term) => {
                if !term.attributes.is_empty() {
                    return Err(Problem::Attributes);
                }
                (Key::Term(checked_name(term.id.name)?), &term.value)
            }
            Entry::Comment(_) | Entry::GroupComment(_) | Entry::ResourceComment(_) => continue,
            Entry::Junk { .. } => return Err(Problem::Entry),
        };
        if shapes.len() >= MAX_KEYS {
            return Err(Problem::Budget(Budget::Keys));
        }
        let mut shape = Shape::default();
        scan(pattern, &mut shape, PatternRole::Message)
            .map_err(|cause| Problem::Contract(Sealed::new(cause)))?;
        if shapes.insert(key, shape).is_some() {
            return Err(Problem::Duplicate);
        }
    }
    Ok(shapes)
}

fn checked_contracts(shapes: &BTreeMap<Key, Shape>) -> Result<BTreeMap<Key, Contract>, Problem> {
    let mut contracts = BTreeMap::new();
    for key in shapes.keys() {
        let contract = expanded(key, |key| shapes.get(key))
            .map_err(|cause| Problem::Contract(Sealed::new(cause)))?;
        if contract.variables().len() > MAX_ARGUMENTS {
            return Err(Problem::Budget(Budget::Arguments));
        }
        if contract
            .variables()
            .keys()
            .any(|name| name.as_str().len() > MAX_NAME_BYTES)
        {
            return Err(Problem::Budget(Budget::Name));
        }
        let _prior = contracts.insert(key.clone(), contract);
    }
    Ok(contracts)
}

fn validate_translation(english: &Parsed, localized: &Parsed) -> Result<(), Problem> {
    for (key, contract) in localized.contracts.iter() {
        match key {
            Key::Message(_) => {
                let expected = english.contracts.get(key).ok_or(Problem::MissingMessage)?;
                compare(key, expected, contract, VariableCoverage::Subset)
                    .map_err(|cause| Problem::Contract(Sealed::new(cause)))?;
            }
            Key::Term(_) => {}
        }
    }
    Ok(())
}

/// Owned resources and shared core functions, with only already checked message
/// contracts. Format before publishing retained Caption/ThemeName/ContentText.
pub(super) struct Formatter {
    bundles: Bundles<Arc<FluentResource>>,
    english: Arc<BTreeMap<Key, Contract>>,
}

impl std::fmt::Debug for Formatter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Formatter").finish_non_exhaustive()
    }
}

impl Formatter {
    pub(super) fn caption(
        &self,
        key: &MessageKey,
        arguments: &Arguments,
    ) -> Result<Caption, Problem> {
        self.resolve(key, arguments, str::parse)
    }

    pub(super) fn theme_name(
        &self,
        key: &MessageKey,
        arguments: &Arguments,
    ) -> Result<ThemeName, Problem> {
        self.resolve(key, arguments, str::parse)
    }

    pub(super) fn content(
        &self,
        key: &MessageKey,
        arguments: &Arguments,
    ) -> Result<ContentText, Problem> {
        self.resolve(key, arguments, str::parse)
    }

    fn resolve<T>(
        &self,
        key: &MessageKey,
        arguments: &Arguments,
        validate: impl Fn(&str) -> Result<T, vessel_proto::plugins::InvalidPluginValue>,
    ) -> Result<T, Problem> {
        let contract = self
            .english
            .get(&key.key())
            .ok_or(Problem::MissingMessage)?;
        check_arguments(contract, arguments)?;
        let args = fluent_arguments(arguments)?;
        let attempt = |bundle| {
            format_one(bundle, key.0.as_str(), &args)
                .and_then(|text| validate(&text).map_err(Problem::Output))
        };
        match attempt(&self.bundles.localized) {
            Ok(value) => Ok(value),
            Err(translation) => {
                attempt(&self.bundles.fallback).map_err(|english| Problem::Fallback {
                    translation: Box::new(translation),
                    english: Box::new(english),
                })
            }
        }
    }
}

fn check_arguments(contract: &Contract, arguments: &Arguments) -> Result<(), Problem> {
    if contract.variables().len() != arguments.0.len() {
        return Err(Problem::Arguments);
    }
    for (name, kind) in contract.variables() {
        let value = arguments.0.get(name).ok_or(Problem::Arguments)?;
        match (kind, value) {
            (Kind::Text, Argument::Text(_) | Argument::Number(_))
            | (Kind::Choice(_), Argument::Text(_))
            | (Kind::Number, Argument::Number(_))
            | (Kind::Date, Argument::Date(_)) => {}
            (Kind::Text, Argument::Date(_))
            | (Kind::Choice(_), Argument::Number(_) | Argument::Date(_))
            | (Kind::Number, Argument::Text(_) | Argument::Date(_))
            | (Kind::Date, Argument::Text(_) | Argument::Number(_)) => {
                return Err(Problem::Arguments);
            }
        }
    }
    Ok(())
}

fn fluent_arguments(arguments: &Arguments) -> Result<FluentArgs<'_>, Problem> {
    let mut args = FluentArgs::new();
    for (name, value) in &arguments.0 {
        let value = match value {
            Argument::Text(text) => FluentValue::from(text.as_str()),
            Argument::Number(number) => number.clone().fluent(),
            Argument::Date(date) => FluentValue::from(
                date.fluent()
                    .map_err(|cause| Problem::Date(Sealed::new(cause)))?,
            ),
        };
        args.set(name.as_str(), value);
    }
    Ok(args)
}

fn format_one(
    bundle: &fluent_bundle::concurrent::FluentBundle<Arc<FluentResource>>,
    key: &str,
    args: &FluentArgs<'_>,
) -> Result<String, Problem> {
    let pattern = bundle
        .get_message(key)
        .and_then(|message| message.value())
        .ok_or(Problem::MissingMessage)?;
    let mut errors = Vec::new();
    let mut value = BoundedText::default();
    let formatted = bundle.write_pattern(&mut value, pattern, Some(args), &mut errors);
    if formatted.is_err() {
        return Err(Problem::Budget(Budget::Output));
    }
    if errors.is_empty() {
        Ok(value.0)
    } else {
        Err(Problem::Bundle(BundleFailure(errors)))
    }
}

#[derive(Default)]
struct BoundedText(String);

impl std::fmt::Write for BoundedText {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        if self
            .0
            .len()
            .checked_add(text.len())
            .is_none_or(|bytes| bytes > MAX_FORMATTED_BYTES)
        {
            return Err(std::fmt::Error);
        }
        self.0.push_str(text);
        Ok(())
    }
}
