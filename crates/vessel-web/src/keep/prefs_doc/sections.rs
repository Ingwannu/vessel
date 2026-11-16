//! The preferences file's sections as this build reads them: each a
//! struct of [`Field`]s that keeps its unknown keys (`rest`), and the
//! values the page is given for it. A machine's defaults keep each agent's
//! entry whole ([`StoredAgent`]).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use vessel_proto::{ChoiceValue, HarnessKind, ProfileName, RawJson};

use super::super::lenient::{Blank, Entries, Field};
use crate::wire::{
    AgentDefaults, AgentKind, Alerts, Appearance, ChoiceKey, Choices, CodeSize, Composer,
    ContentWidth, ConversationDisplay, Grouping, InterfaceSize, LabelPicks, Layout, MachineDefaults,
    Notifications, Panel, SendKey, SessionSort, SidebarView, Theme, TurnDelivery,
};

/// Declares a section of plain settings: each a [`Field`], the section's
/// unknown keys kept in `rest`, and its values for the page.
macro_rules! section {
    ($(#[$meta:meta])* $name:ident => $wire:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(default)]
        pub(super) struct $name {
            $(
                #[serde(skip_serializing_if = "Field::is_absent")]
                pub(super) $field: Field<$ty>,
            )*
            #[serde(flatten)]
            rest: BTreeMap<String, RawJson>,
        }

        impl $name {
            /// The values, a default for each that is not read.
            pub(super) fn values(&self) -> $wire {
                let defaults = $wire::default();
                $wire { $($field: self.$field.value_or(defaults.$field),)* }
            }
        }

        impl Blank for $name {
            fn is_blank(&self) -> bool {
                $(self.$field.is_absent() &&)* self.rest.is_empty()
            }
        }
    };
}

section! {
    /// `appearance`.
    AppearanceFile => Appearance {
        theme: Theme,
        interface_size: InterfaceSize,
        code_size: CodeSize,
        content_width: ContentWidth,
        conversation: ConversationDisplay,
    }
}

section! {
    /// `layout`.
    LayoutFile => Layout { sidebar: Panel, inspector: Panel }
}

section! {
    /// `composer`.
    ComposerFile => Composer { send: SendKey, delivery: TurnDelivery }
}

section! {
    /// `notifications`.
    NotificationsFile => Notifications { alerts: Alerts, title: bool }
}

section! {
    /// `sidebar`.
    SidebarFile => SidebarView {
        group: Grouping,
        sort: SessionSort,
        ended: bool,
        labels: LabelPicks,
    }
}

/// One machine's defaults in the file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct MachineFile {
    #[serde(skip_serializing_if = "Field::is_absent")]
    pub(super) agent: Field<Option<HarnessKind>>,
    #[serde(skip_serializing_if = "Field::is_blank")]
    pub(super) agents: Field<Entries<AgentKind, StoredAgent>>,
    #[serde(flatten)]
    rest: BTreeMap<String, RawJson>,
}

impl MachineFile {
    pub(super) fn values(&self) -> MachineDefaults {
        MachineDefaults {
            agent: self.agent.read().copied().flatten(),
            agents: self
                .agents
                .read()
                .map(|agents| {
                    agents
                        .read()
                        .iter()
                        .map(|(kind, stored)| (*kind, stored.0.clone()))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

impl Blank for MachineFile {
    fn is_blank(&self) -> bool {
        self.agent.is_absent() && self.agents.is_blank() && self.rest.is_empty()
    }
}

/// One agent's defaults in the file: read whole or kept whole. Anything
/// it does not know (a field, a choice of an unknown kind, a value past
/// the bounds) keeps the entry as it was, so a newer page's entry is not
/// cut down to what this build understands.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "AgentRecord")]
pub(super) struct StoredAgent(pub(super) AgentDefaults);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentRecord {
    #[serde(default)]
    profile: Option<ProfileName>,
    #[serde(default)]
    choices: BTreeMap<ChoiceKey, ChoiceValue>,
}

impl TryFrom<AgentRecord> for StoredAgent {
    type Error = crate::wire::ChoicesError;

    fn try_from(record: AgentRecord) -> Result<Self, Self::Error> {
        Ok(Self(AgentDefaults {
            profile: record.profile,
            choices: Choices::strict(record.choices)?,
        }))
    }
}

impl Serialize for StoredAgent {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}
