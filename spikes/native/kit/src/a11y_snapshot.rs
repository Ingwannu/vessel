//! Test support: the nodes of GPUI's real accessibility snapshot, by role,
//! name and the few states a control says (TRACKER 808: toggled, expanded,
//! the actions it takes). No raw snapshot is printed.
use gpui::{VisualTestContext, Window};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Aria {
    role: String,
    label: Option<String>,
    toggled: Option<String>,
    expanded: Option<bool>,
    selected: Option<bool>,
    #[serde(default)]
    on_action: Vec<String>,
}

/// One node as assistive technology meets it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct A11yNode {
    pub(crate) label: Option<String>,
    /// "True" or "False" when it says a checked state.
    pub(crate) toggled: Option<String>,
    pub(crate) expanded: Option<bool>,
    pub(crate) selected: Option<bool>,
    /// The actions it takes ("Click", "Focus").
    pub(crate) actions: Vec<String>,
}

#[derive(Deserialize)]
struct Node {
    aria: Aria,
}

#[derive(Deserialize)]
struct Snapshot {
    nodes: BTreeMap<String, Node>,
}

/// Turn on the window's real accessibility adapter (gpui-pre change 11); the
/// caller then draws a frame.
pub(crate) fn activate(cx: &mut VisualTestContext) {
    let handle = cx.update(|window, _| window.window_handle());
    assert!(cx.simulate_accessibility_activation(handle));
    cx.run_until_parked();
}

/// The name of every node with the button role, sorted; `None` while the
/// adapter is inactive.
pub(crate) fn button_names(window: &Window) -> Option<Vec<Option<String>>> {
    let snapshot: Snapshot = serde_json::from_str(&window.debug_a11y_tree_json()?).ok()?;
    let mut names: Vec<Option<String>> = snapshot
        .nodes
        .into_values()
        .filter(|node| node.aria.role == "Button")
        .map(|node| node.aria.label)
        .collect();
    names.sort();
    Some(names)
}

/// Every node with `role`, by name; `None` while the adapter is inactive.
pub(crate) fn nodes(window: &Window, role: &str) -> Option<Vec<A11yNode>> {
    let snapshot: Snapshot = serde_json::from_str(&window.debug_a11y_tree_json()?).ok()?;
    let mut nodes: Vec<A11yNode> = snapshot
        .nodes
        .into_values()
        .filter(|node| node.aria.role == role)
        .map(|node| A11yNode {
            label: node.aria.label,
            toggled: node.aria.toggled,
            expanded: node.aria.expanded,
            selected: node.aria.selected,
            actions: node.aria.on_action,
        })
        .collect();
    nodes.sort_by(|a, b| a.label.cmp(&b.label));
    Some(nodes)
}
