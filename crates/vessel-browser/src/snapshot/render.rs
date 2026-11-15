//! Complete escaped lines, with a reserved visible truncation marker.
use super::SnapshotError;
use super::graph::{Checked, Graph, Node, State};
use vessel_proto::browser::{CappedText, MAX_PAGE_TEXT_BYTES, PageText, TextCompleteness};
const TRUNCATION_MARKER: &str = "- text: \"Snapshot truncated.\"";

pub(super) fn render(graph: &Graph) -> Result<CappedText, SnapshotError> {
    let mut result = String::new();
    let mut truncated = graph.truncated;
    for node in &graph.nodes.0 {
        let line = line(node)?;
        let reserved = result
            .len()
            .saturating_add(line.len())
            .saturating_add(TRUNCATION_MARKER.len())
            .saturating_add(2);
        if reserved > MAX_PAGE_TEXT_BYTES {
            truncated = true;
            break;
        }
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(&line);
    }
    if truncated {
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(TRUNCATION_MARKER);
    }
    let text = PageText::try_from(result).map_err(|_over_budget| SnapshotError::Graph)?;
    Ok(CappedText {
        text,
        completeness: if truncated {
            TextCompleteness::Truncated
        } else {
            TextCompleteness::Whole
        },
    })
}
fn line(node: &Node) -> Result<String, SnapshotError> {
    let indent = "  ".repeat(usize::from(node.depth()));
    match node {
        Node::Text { text, .. } => Ok(format!("{indent}- text: {}", quote(text.as_str())?)),
        Node::Element {
            role,
            name,
            reference,
            state,
            ..
        } => {
            let mut line = format!("{indent}- {}", role.name());
            if !name.as_str().is_empty() {
                line.push(' ');
                line.push_str(&quote(name.as_str())?);
            }
            let mut attributes = state_attributes(state);
            if let Some(reference) = reference {
                attributes.push(format!("ref={}", reference.as_str()));
            }
            if !attributes.is_empty() {
                line.push_str(" [");
                line.push_str(&attributes.join(" "));
                line.push(']');
            }
            Ok(line)
        }
    }
}
fn quote(text: &str) -> Result<String, SnapshotError> {
    serde_json::to_string(text).map_err(SnapshotError::Render)
}
fn state_attributes(state: &State) -> Vec<String> {
    let mut attributes = Vec::new();
    if let Some(checked) = &state.checked {
        let value = match checked {
            Checked::True => "true",
            Checked::False => "false",
            Checked::Mixed => "mixed",
        };
        attributes.push(format!("checked={value}"));
    }
    for (name, value) in [("expanded", state.expanded), ("selected", state.selected)] {
        if let Some(value) = value {
            attributes.push(format!("{name}={value}"));
        }
    }
    if state.disabled {
        attributes.push("disabled".to_owned());
    }
    if state.readonly {
        attributes.push("readonly".to_owned());
    }
    if let Some(level) = &state.level {
        attributes.push(format!("level={}", level.get()));
    }
    attributes
}
