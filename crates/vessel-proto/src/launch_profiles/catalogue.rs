//! Bounded driver-neutral editor options; unknown controls are never invented.

use super::{InvalidLaunchValue, LaunchPayload, LaunchSelection, codec};
use crate::{
    OptionCategory, OptionId, OptionKind, OptionValue, OptionValueId, ProfileName, SessionOption,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, de};
use std::{collections::HashSet, sync::Arc};

/// A complete, bounded catalogue of the agent's actual option controls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(as = "Vec<SessionOption>"))]
pub struct LaunchCatalogueOptions(Arc<[SessionOption]>);

impl LaunchCatalogueOptions {
    /// Most controls in one successful whole reply.
    pub const MAX_OPTIONS: usize = 64;
    /// Most values in one select; decoding rejects the next before constructing it.
    pub const MAX_VALUES: usize = 128;
    /// Byte limit for reported labels and group names.
    pub const MAX_LABEL_BYTES: usize = 512;
    /// Byte limit for a reported explanation.
    pub const MAX_DESCRIPTION_BYTES: usize = 4_096;
    /// Authoritative controls, borrowed without permitting mutation.
    #[must_use]
    pub fn as_slice(&self) -> &[SessionOption] {
        &self.0
    }
}

impl TryFrom<Vec<SessionOption>> for LaunchCatalogueOptions {
    type Error = InvalidLaunchValue;
    fn try_from(options: Vec<SessionOption>) -> Result<Self, Self::Error> {
        if options.len() > Self::MAX_OPTIONS {
            return Err(InvalidLaunchValue::Catalogue);
        }
        let mut ids = HashSet::new();
        for option in &options {
            validate(option)?;
            if !ids.insert(&option.id) {
                return Err(InvalidLaunchValue::Catalogue);
            }
        }
        let value = Self(Arc::from(options));
        LaunchPayload::check(&value)?;
        Ok(value)
    }
}

fn text(text: &str, cap: usize) -> Result<(), InvalidLaunchValue> {
    if text.len() > cap
        || text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\r' | '\n' | '\t'))
    {
        Err(InvalidLaunchValue::Catalogue)
    } else {
        Ok(())
    }
}
fn description(value: Option<&str>) -> Result<(), InvalidLaunchValue> {
    value.map_or(Ok(()), |value| {
        text(value, LaunchCatalogueOptions::MAX_DESCRIPTION_BYTES)
    })
}
fn validate(option: &SessionOption) -> Result<(), InvalidLaunchValue> {
    super::choices::id(option.id.as_str())?;
    text(&option.name, LaunchCatalogueOptions::MAX_LABEL_BYTES)?;
    description(option.description.as_deref())?;
    match &option.kind {
        OptionKind::Select { current, values } => {
            super::choices::id(current.as_str())?;
            if values.len() > LaunchCatalogueOptions::MAX_VALUES {
                return Err(InvalidLaunchValue::Catalogue);
            }
            let mut ids = HashSet::new();
            for value in values {
                super::choices::id(value.id.as_str())?;
                text(&value.name, LaunchCatalogueOptions::MAX_LABEL_BYTES)?;
                description(value.description.as_deref())?;
                if let Some(group) = &value.group {
                    text(group, LaunchCatalogueOptions::MAX_LABEL_BYTES)?;
                }
                if !ids.insert(&value.id) {
                    return Err(InvalidLaunchValue::Catalogue);
                }
            }
        }
        OptionKind::Toggle { .. } | OptionKind::Unknown => {}
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EncodedValue {
    id: OptionValueId,
    name: String,
    #[serde(default, deserialize_with = "codec::optional")]
    description: Option<String>,
    #[serde(default, deserialize_with = "codec::optional")]
    group: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Control {
    Select,
    Toggle,
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EncodedOption {
    id: OptionId,
    name: String,
    #[serde(default, deserialize_with = "codec::optional")]
    description: Option<String>,
    category: OptionCategory,
    kind: Control,
    #[serde(default, deserialize_with = "codec::optional")]
    current: Option<OptionValueId>,
    #[serde(default, deserialize_with = "values")]
    values: Option<Vec<EncodedValue>>,
    #[serde(default, deserialize_with = "codec::optional")]
    on: Option<bool>,
}

fn values<'de, D: Deserializer<'de>>(input: D) -> Result<Option<Vec<EncodedValue>>, D::Error> {
    codec::bounded::<D, EncodedValue, { LaunchCatalogueOptions::MAX_VALUES }>(input).map(Some)
}
impl EncodedOption {
    fn into_option(self) -> Result<SessionOption, InvalidLaunchValue> {
        let kind = match (self.kind, self.current, self.values, self.on) {
            (Control::Select, Some(current), Some(values), None) => OptionKind::Select {
                current,
                values: values
                    .into_iter()
                    .map(|value| OptionValue {
                        id: value.id,
                        name: value.name,
                        description: value.description,
                        group: value.group,
                    })
                    .collect(),
            },
            (Control::Toggle, None, None, Some(on)) => OptionKind::Toggle { on },
            (Control::Unknown, None, None, None) => OptionKind::Unknown,
            (Control::Select | Control::Toggle | Control::Unknown, _, _, _) => {
                return Err(InvalidLaunchValue::Catalogue);
            }
        };
        Ok(SessionOption {
            id: self.id,
            name: self.name,
            description: self.description,
            category: self.category,
            kind,
        })
    }
}
impl<'de> Deserialize<'de> for LaunchCatalogueOptions {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        let encoded = codec::bounded::<D, EncodedOption, { Self::MAX_OPTIONS }>(input)?;
        let options: Vec<SessionOption> = encoded
            .into_iter()
            .map(EncodedOption::into_option)
            .collect::<Result<_, _>>()
            .map_err(de::Error::custom)?;
        Self::try_from(options).map_err(de::Error::custom)
    }
}

/// The actual account association, reported by the node without credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum LaunchAccountKind {
    /// The harness's real default account; absence of a selector means this one.
    Default,
    /// A saved named account; absence must never silently select it.
    Named,
    /// A future association is not authority to guess an account.
    #[serde(other)]
    Unknown,
}

/// Canonical account name and default status, not its secret configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct LaunchAccount {
    name: ProfileName,
    kind: LaunchAccountKind,
}
impl LaunchAccount {
    /// Record actual retained node facts, rather than infer defaults in clients.
    #[must_use]
    pub const fn new(name: ProfileName, kind: LaunchAccountKind) -> Self {
        Self { name, kind }
    }
    /// Canonical name selected by the node's existing account owner.
    #[must_use]
    pub const fn name(&self) -> &ProfileName {
        &self.name
    }
    /// Whether an omitted selector names this actual account.
    #[must_use]
    pub const fn kind(&self) -> LaunchAccountKind {
        self.kind
    }
}

/// One terminal probe result, echoed with its exact request key for stale isolation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchCatalogue {
    selection: LaunchSelection,
    account: LaunchAccount,
    options: LaunchCatalogueOptions,
    probed_at: DateTime<Utc>,
}
impl LaunchCatalogue {
    /// Refuse the whole result if combined escaping exceeds the payload budget.
    ///
    /// # Errors
    /// The complete escaped selection/account/options body exceeds its cap.
    pub fn new(
        selection: LaunchSelection,
        account: LaunchAccount,
        options: LaunchCatalogueOptions,
        probed_at: DateTime<Utc>,
    ) -> Result<Self, InvalidLaunchValue> {
        let value = Self {
            selection,
            account,
            options,
            probed_at,
        };
        LaunchPayload::check(&value)?;
        Ok(value)
    }
    /// The exact harness/account/ordered choices used for this probe.
    #[must_use]
    pub const fn selection(&self) -> &LaunchSelection {
        &self.selection
    }
    /// Canonical account facts from the same retained node snapshot.
    #[must_use]
    pub const fn account(&self) -> &LaunchAccount {
        &self.account
    }
    /// Actual reported options after the selected choices applied.
    #[must_use]
    pub const fn options(&self) -> &LaunchCatalogueOptions {
        &self.options
    }
    /// Time the completed, cleaned-up probe was observed.
    #[must_use]
    pub const fn probed_at(&self) -> DateTime<Utc> {
        self.probed_at
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EncodedCatalogue {
    selection: LaunchSelection,
    account: LaunchAccount,
    options: LaunchCatalogueOptions,
    probed_at: DateTime<Utc>,
}
impl<'de> Deserialize<'de> for LaunchCatalogue {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        let raw: EncodedCatalogue = codec::decode(input)?;
        Self::new(raw.selection, raw.account, raw.options, raw.probed_at).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests;
