//! Installation-local Fluent data; core Translator never receives these keys.

mod errors;
mod resources;

use std::{collections::BTreeMap, fmt, str::FromStr, sync::Arc};

use crate::fluent_contract::{Key, Name};
use crate::localization::{Date, Number};

use errors::Sealed;
use errors::{Budget, Problem};
use resources::{Formatter, Resources};

const MAX_LOCALES: usize = 16;
const MAX_SOURCE_BYTES: usize = 32 * 1024;
const MAX_RESOURCE_BYTES: usize = 256 * 1024;
const MAX_KEYS: usize = 256;
const MAX_NAME_BYTES: usize = 128;
const MAX_ARGUMENTS: usize = 16;
const MAX_ZONE_BYTES: usize = 128;
// A conservative source resource budget, not a Fluent parser/formatter: the
// number of raw opening delimiters bounds possible parser/AST recursion even
// when strings/comments contain misleading closing delimiters. Literal-heavy
// resources can hit this earlier; the author-port difference is intentional.
const MAX_SYNTAX_OPENERS: usize = 128;
const MAX_FORMATTED_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct LocaleTag(String);

impl FromStr for LocaleTag {
    type Err = Problem;
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        if raw.len() > MAX_NAME_BYTES {
            return Err(Problem::Budget(Budget::Name));
        }
        let language: unic_langid::LanguageIdentifier = raw.parse().map_err(Problem::Locale)?;
        Ok(Self(language.to_string()))
    }
}

pub(super) struct Resource {
    locale: LocaleTag,
    source: Arc<str>,
}

impl Resource {
    pub(super) fn new(locale: LocaleTag, source: &str) -> Result<Self, Problem> {
        if source.len() > MAX_SOURCE_BYTES {
            return Err(Problem::Budget(Budget::Source));
        }
        if source
            .chars()
            .filter(|c| matches!(c, '{' | '('))
            .take(MAX_SYNTAX_OPENERS.saturating_add(1))
            .count()
            > MAX_SYNTAX_OPENERS
        {
            return Err(Problem::Budget(Budget::Syntax));
        }
        Ok(Self {
            locale,
            source: Arc::from(source),
        })
    }
}

impl fmt::Debug for Resource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Resource")
            .field("locale", &self.locale)
            .field("bytes", &self.source.len())
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct MessageKey(Name);

impl FromStr for MessageKey {
    type Err = Problem;
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        checked_name(raw).map(Self)
    }
}

fn checked_name(raw: &str) -> Result<Name, Problem> {
    if raw.len() > MAX_NAME_BYTES {
        return Err(Problem::Budget(Budget::Name));
    }
    Name::parse(raw).map_err(|cause| Problem::Contract(Sealed::new(cause)))
}

impl MessageKey {
    fn key(&self) -> Key {
        Key::Message(self.0.clone())
    }
}

#[derive(Debug, Clone)]
pub(super) enum Argument {
    Text(vessel_proto::plugins::ContentText),
    Number(Number),
    Date(Date),
}

#[derive(Clone, Default)]
pub(super) struct Arguments(BTreeMap<Name, Argument>);

impl fmt::Debug for Arguments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Arguments(<private values>)")
    }
}

impl Arguments {
    pub(super) fn new(values: Vec<(&str, Argument)>) -> Result<Self, Problem> {
        if values.len() > MAX_ARGUMENTS {
            return Err(Problem::Budget(Budget::Arguments));
        }
        let mut args = BTreeMap::new();
        for (name, value) in values {
            if let Argument::Date(Date::Time { zone, .. } | Date::DateTime { zone, .. }) = &value {
                if zone.len() > MAX_ZONE_BYTES || zone.chars().any(char::is_control) {
                    return Err(Problem::Arguments);
                }
            }
            if args.insert(checked_name(name)?, value).is_some() {
                return Err(Problem::Duplicate);
            }
        }
        Ok(Self(args))
    }
}

#[cfg(test)]
mod tests;
