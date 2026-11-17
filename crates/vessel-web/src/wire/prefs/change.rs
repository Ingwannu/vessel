//! One change to the preferences ([`PreferenceChange`]): each names one
//! setting and its new value, so two tabs changing different settings
//! both keep their change, and the same setting ends as the later change
//! set it.

#[cfg(feature = "ts")]
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use vessel_proto::{HarnessKind, NodeId, ProfileName};

use super::{
    ActionId, Alerts, Bindings, Choices, CodeSize, ContentWidth, ConversationDisplay, Grouping,
    InterfaceSize, Panel, SendKey, SessionSort, ShortcutEdits, Theme, TurnDelivery,
};
#[cfg(doc)]
use super::{
    Appearance, Composer, Layout, MAX_BINDINGS, MAX_CHOICE_TEXT, MAX_CHOICES, MAX_EDITS,
    Notifications, SidebarView,
};
#[cfg(feature = "ts")]
use super::{ChoiceKey, KeyBinding};

/// One change to the preferences: each replaces one setting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "set", content = "value", rename_all = "snake_case")]
pub enum PreferenceChange {
    /// [`Appearance::theme`].
    Theme(Theme),
    /// [`Appearance::interface_size`].
    InterfaceSize(InterfaceSize),
    /// [`Appearance::code_size`].
    CodeSize(CodeSize),
    /// [`Appearance::content_width`].
    ContentWidth(ContentWidth),
    /// [`Appearance::conversation`].
    Conversation(ConversationDisplay),
    /// [`Layout::sidebar`].
    SidebarPanel(Panel),
    /// [`Layout::inspector`].
    InspectorPanel(Panel),
    /// [`Composer::send`].
    Send(SendKey),
    /// [`Composer::delivery`].
    Delivery(TurnDelivery),
    /// [`Notifications::alerts`].
    Alerts(Alerts),
    /// [`Notifications::title`].
    TitleCount(bool),
    /// [`SidebarView::group`].
    SidebarGroup(Grouping),
    /// [`SidebarView::sort`].
    SidebarSort(SessionSort),
    /// [`SidebarView::ended`].
    SidebarEnded(bool),
    /// [`SidebarView::labels`]: the whole filter, at most 32 names.
    SidebarLabels {
        /// The names picked; `""` is Unlabelled.
        #[cfg_attr(feature = "ts", ts(as = "Vec<String>"))]
        picks: super::LabelPicks,
    },
    /// Keys for some actions, applied together, so a key moves from one
    /// action to another in one change.
    Shortcuts {
        /// At most [`MAX_EDITS`].
        #[cfg_attr(feature = "ts", ts(as = "Vec<ShortcutEdit>"))]
        edits: ShortcutEdits,
    },
    /// Every action back to its default keys (actions this page does not
    /// know included).
    ResetShortcuts,
    /// The agent a new session on `node` picks first.
    DefaultAgent {
        /// The machine.
        node: NodeId,
        /// The agent; `None` forgets it.
        agent: Option<HarnessKind>,
    },
    /// What `agent` starts with on `node`, replacing what was remembered.
    AgentDefaults {
        /// The machine.
        node: NodeId,
        /// The agent.
        agent: HarnessKind,
        /// The profile; `None` is the harness's default.
        profile: Option<ProfileName>,
        /// The agent's options, at most [`MAX_CHOICES`]; an id or a value
        /// longer than [`MAX_CHOICE_TEXT`] bytes is not remembered.
        #[cfg_attr(
            feature = "ts",
            ts(as = "BTreeMap<ChoiceKey, vessel_proto::ChoiceValue>")
        )]
        choices: Choices,
    },
    /// Forget everything remembered for `node`.
    ForgetDefaults {
        /// The machine.
        node: NodeId,
    },
}

/// One action's new keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ShortcutEdit {
    /// The action.
    pub action: ActionId,
    /// Its keys from now on.
    pub to: ShortcutTarget,
}

/// What an action's keys become.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ShortcutTarget {
    /// These keys (empty: none at all).
    Bindings {
        /// At most [`MAX_BINDINGS`].
        #[cfg_attr(feature = "ts", ts(as = "Vec<KeyBinding>"))]
        keys: Bindings,
    },
    /// The action's default keys.
    Default,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_are_tagged_by_setting_with_their_value() {
        let change: PreferenceChange =
            serde_json::from_str(r#"{"set":"theme","value":"dark"}"#).unwrap();
        assert_eq!(change, PreferenceChange::Theme(Theme::Dark));
        let change: PreferenceChange =
            serde_json::from_str(r#"{"set":"delivery","value":"steer"}"#).unwrap();
        assert_eq!(change, PreferenceChange::Delivery(TurnDelivery::Steer));
        let change: PreferenceChange =
            serde_json::from_str(r#"{"set":"reset_shortcuts"}"#).unwrap();
        assert_eq!(change, PreferenceChange::ResetShortcuts);
        let change: PreferenceChange = serde_json::from_str(
            r#"{"set":"shortcuts","value":{"edits":[{"action":"palette","to":{"kind":"bindings","keys":["Mod+K","/"]}},{"action":"find","to":{"kind":"default"}}]}}"#,
        )
        .unwrap();
        let PreferenceChange::Shortcuts { edits } = change else {
            panic!("{change:?}");
        };
        assert_eq!(edits.items().len(), 2);
        for refused in [
            r#"{"set":"interface_size","value":21}"#,
            r#"{"set":"code_size","value":9}"#,
            r#"{"set":"theme","value":"sepia"}"#,
            r#"{"set":"delivery","value":"start"}"#,
            r#"{"set":"shortcuts","value":{"edits":[{"action":"palette","to":{"kind":"bindings","keys":["Mod+K","/","N"]}}]}}"#,
            r#"{"set":"shortcuts","value":{"edits":[{"action":"palette","to":{"kind":"bindings","keys":["mod+k"]}}]}}"#,
            r#"{"set":"what","value":1}"#,
        ] {
            assert!(
                serde_json::from_str::<PreferenceChange>(refused).is_err(),
                "{refused}"
            );
        }
        let seventeen: Vec<String> = (0..17)
            .map(|n| format!(r#"{{"action":"a{n}","to":{{"kind":"default"}}}}"#))
            .collect();
        let text = format!(
            r#"{{"set":"shortcuts","value":{{"edits":[{}]}}}}"#,
            seventeen.join(",")
        );
        assert!(serde_json::from_str::<PreferenceChange>(&text).is_err());
    }
}
