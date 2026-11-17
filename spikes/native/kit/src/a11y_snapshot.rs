//! Test support: the button nodes of GPUI's real accessibility snapshot, by
//! role and name only. No raw snapshot is printed.
use gpui::{VisualTestContext, Window};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Aria {
    role: String,
    label: Option<String>,
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
