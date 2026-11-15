//! Bounded provider launch inputs. Debug intentionally omits arguments and values.
//! Literal values are local configuration; remote snapshots never include them.

use crate::{ConfigValue, EnvName, agents::AgentProgram};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};
use std::{str::FromStr, sync::Arc};

/// Resource limit for argv, environment and denied-tool entries.
pub const MAX_LAUNCH_ITEMS: usize = 256;
/// Maximum bytes in one argument or literal environment value.
pub const MAX_LAUNCH_TEXT: usize = 8192;

/// Invalid provider launch inputs are refused without echoing values.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LaunchConfigError {
    /// Prevent resource growth at the edge.
    #[error("provider launch list exceeds {MAX_LAUNCH_ITEMS} entries")]
    TooMany,
    /// Controls cannot safely cross runtime or diagnostic boundaries.
    #[error("provider launch text exceeds {MAX_LAUNCH_TEXT} bytes or contains a control character")]
    Text,
    /// Windows environment names are case insensitive on every input platform.
    #[error("provider environment repeats a name ignoring case")]
    DuplicateVariable,
}

/// Validated literal arguments; they are never interpreted as shell text.
#[derive(Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "RawArguments", into = "Vec<String>")]
pub struct Arguments(Vec<String>);

impl Arguments {
    /// Check the vector before it can enter a command.
    ///
    /// # Errors
    /// `LaunchConfigError` rejects oversized input or control characters.
    pub fn new(args: Vec<String>) -> Result<Self, LaunchConfigError> {
        if args.len() > MAX_LAUNCH_ITEMS {
            return Err(LaunchConfigError::TooMany);
        }
        for arg in &args {
            text(arg)?;
        }
        Ok(Self(args))
    }

    /// Literal argv entries, including intentionally empty append arguments.
    #[must_use]
    pub fn all(&self) -> &[String] {
        &self.0
    }
}

impl fmt::Debug for Arguments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Arguments")
            .field("count", &self.0.len())
            .finish()
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct RawArguments(#[serde(deserialize_with = "read_list")] Vec<String>);

impl TryFrom<RawArguments> for Arguments {
    type Error = LaunchConfigError;
    fn try_from(raw: RawArguments) -> Result<Self, Self::Error> {
        Self::new(raw.0)
    }
}

impl From<Arguments> for Vec<String> {
    fn from(args: Arguments) -> Self {
        args.0
    }
}

/// The provider launch edit, not a shell command string.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum CommandEdit {
    /// Use the provider's own command.
    #[default]
    Default,
    /// Keep its executable and append these arguments.
    Append {
        /// Validated argv entries, never shell syntax.
        args: Arguments,
    },
    /// Replace both program and argument vector.
    Replace {
        /// Executable name or absolute path, resolved by Lookup later.
        program: AgentProgram,
        /// Validated argv entries.
        args: Arguments,
    },
}

/// Configuration overlay; Debug reveals only names, never literal values.
#[derive(Clone, PartialEq, Eq, Default, Serialize)]
#[serde(transparent)]
pub struct ProviderEnvironment(BTreeMap<EnvName, ConfigValue>);

impl ProviderEnvironment {
    /// Validate a local overlay independently from any credential resolution.
    ///
    /// # Errors
    /// Bounds, control characters and case-folded duplicate names are refused.
    pub fn new(values: BTreeMap<EnvName, ConfigValue>) -> Result<Self, LaunchConfigError> {
        if values.len() > MAX_LAUNCH_ITEMS {
            return Err(LaunchConfigError::TooMany);
        }
        let mut seen = BTreeSet::new();
        for (name, value) in &values {
            if !seen.insert(name.as_str().to_ascii_uppercase()) {
                return Err(LaunchConfigError::DuplicateVariable);
            }
            if let ConfigValue::Text(value) = value {
                text(value)?;
            }
        }
        Ok(Self(values))
    }

    /// Borrow configured values; callers must not log or publish them.
    #[must_use]
    pub const fn values(&self) -> &BTreeMap<EnvName, ConfigValue> {
        &self.0
    }
}

impl fmt::Debug for ProviderEnvironment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderEnvironment")
            .field("names", &self.0.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl<'de> Deserialize<'de> for ProviderEnvironment {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Environment;
        impl<'de> serde::de::Visitor<'de> for Environment {
            type Value = ProviderEnvironment;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "at most {MAX_LAUNCH_ITEMS} unique environment entries")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = BTreeMap::new();
                while let Some((name, value)) = map.next_entry::<EnvName, ConfigValue>()? {
                    if values.len() == MAX_LAUNCH_ITEMS {
                        return Err(serde::de::Error::custom(LaunchConfigError::TooMany));
                    }
                    if values.insert(name, value).is_some() {
                        return Err(serde::de::Error::custom(
                            LaunchConfigError::DuplicateVariable,
                        ));
                    }
                }
                ProviderEnvironment::new(values).map_err(serde::de::Error::custom)
            }
        }
        de.deserialize_map(Environment)
    }
}

/// A checked native tool pattern, distinct from command arguments and model IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolPattern(Arc<str>);

impl ToolPattern {
    /// Native pattern text for a compatible tool-policy encoder only.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ToolPattern {
    type Err = LaunchConfigError;
    fn from_str(pattern: &str) -> Result<Self, Self::Err> {
        text(pattern)?;
        if pattern.is_empty() {
            return Err(LaunchConfigError::Text);
        }
        Ok(Self(Arc::from(pattern)))
    }
}

/// Bounded literal native tool patterns; omission does not mean enforcement.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "RawArguments", into = "Vec<String>")]
pub struct DisallowedTools(Vec<ToolPattern>);

impl DisallowedTools {
    /// Retain patterns in order for an adapter that can enforce them.
    ///
    /// # Errors
    /// Empty patterns or invalid launch text are refused.
    pub fn new(patterns: Vec<String>) -> Result<Self, LaunchConfigError> {
        if patterns.len() > MAX_LAUNCH_ITEMS {
            return Err(LaunchConfigError::TooMany);
        }
        patterns
            .into_iter()
            .map(|pattern| pattern.parse())
            .collect::<Result<Vec<_>, _>>()
            .map(Self)
    }

    /// Native patterns, without pretending a transport supports them.
    #[must_use]
    pub fn patterns(&self) -> &[ToolPattern] {
        &self.0
    }
}

impl TryFrom<RawArguments> for DisallowedTools {
    type Error = LaunchConfigError;
    fn try_from(raw: RawArguments) -> Result<Self, Self::Error> {
        Self::new(raw.0)
    }
}

impl From<DisallowedTools> for Vec<String> {
    fn from(tools: DisallowedTools) -> Self {
        tools
            .0
            .iter()
            .map(|pattern| pattern.as_str().to_owned())
            .collect()
    }
}

/// Provider configuration owns command, env and denied tools together.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LaunchSettings {
    /// Default/append/replace applies before protocol-specific flags.
    #[serde(default)]
    pub command: CommandEdit,
    /// Configured values; never included in provider snapshots.
    #[serde(default)]
    pub env: ProviderEnvironment,
    /// The selected driver must enforce these or refuse the launch.
    #[serde(default)]
    pub disallowed_tools: DisallowedTools,
}

fn text(value: &str) -> Result<(), LaunchConfigError> {
    if value.len() > MAX_LAUNCH_TEXT || value.chars().any(char::is_control) {
        return Err(LaunchConfigError::Text);
    }
    Ok(())
}

fn read_list<'de, D: serde::Deserializer<'de>>(de: D) -> Result<Vec<String>, D::Error> {
    struct List;
    impl<'de> serde::de::Visitor<'de> for List {
        type Value = Vec<String>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "at most {MAX_LAUNCH_ITEMS} strings")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Self::Value, A::Error> {
            let mut values = Vec::new();
            while let Some(value) = seq.next_element()? {
                if values.len() == MAX_LAUNCH_ITEMS {
                    return Err(serde::de::Error::custom(LaunchConfigError::TooMany));
                }
                values.push(value);
            }
            Ok(values)
        }
    }
    de.deserialize_seq(List)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_and_parsing_rejects_oversize_duplicates_and_nul() {
        let input = r#"{"command":{"mode":"append","args":["synthetic-private-argument"]},"env":{"SYNTHETIC":"synthetic-private-value"}}"#;
        let settings: LaunchSettings = serde_json::from_str(input).unwrap();
        let debug = format!("{settings:?}");
        assert!(!debug.contains("synthetic-private"));
        assert!(debug.contains("SYNTHETIC"));
        for input in [r#"{"Path":"a","PATH":"b"}"#, r#"{"A":"a","A":"b"}"#] {
            assert!(serde_json::from_str::<ProviderEnvironment>(input).is_err());
        }
        assert!(Arguments::new(vec!["x".into(); MAX_LAUNCH_ITEMS + 1]).is_err());
        assert!(Arguments::new(vec!["x".repeat(MAX_LAUNCH_TEXT + 1)]).is_err());
        assert!(Arguments::new(vec!["a\0b".into()]).is_err());
        assert!(DisallowedTools::new(vec![String::new()]).is_err());
        assert!(Arguments::new(vec![String::new()]).is_ok());
    }
}
