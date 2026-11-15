//! Independently written DOM snapshot decoding, below isolated-world admission.
//!
//! The reference specifies behavior only. No reference source/assets are copied
//! or adapted. The owner invalidates first, reserves a never-reused namespace,
//! and clears again on any decode/render failure. None of this qualifies a page.
mod config;
mod graph;
mod render;
pub(crate) use config::{Config, SOURCE};
#[cfg(test)]
mod tests;

use crate::references::{ReferenceError, References, SnapshotNamespace};
use graph::{Graph, Node};
use vessel_proto::browser::{CappedText, ElementRef, SnapshotMetric, SnapshotStats, TextByteCount};

/// Count work while traversing, not just the number of visible output lines.
pub(crate) const MAX_SNAPSHOT_NODES: usize = 1500;
/// Keep the independently authored script and host decoder below recursive-stack limits.
pub(crate) const MAX_SNAPSHOT_DEPTH: u8 = 32;

#[derive(thiserror::Error)]
pub(crate) enum SnapshotError {
    #[error("browser snapshot graph could not be decoded")]
    Decode(#[source] serde_json::Error),
    #[error("browser snapshot reference is inconsistent with its admitted traversal")]
    Reference(#[from] ReferenceError),
    #[error("browser snapshot graph is inconsistent")]
    Graph,
    #[error("browser snapshot rendering could not be encoded")]
    Render(#[source] serde_json::Error),
    #[error("browser snapshot script did not return a valid graph")]
    Evaluation(#[source] crate::cdp::requests::EvaluationError),
}
impl std::fmt::Debug for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SnapshotError({self})")
    }
}
/// Page content and element fingerprints never enter diagnostic formatting.
#[derive(Clone)]
pub(crate) struct Binding(graph::Fingerprint);
impl Binding {
    /// Replaced/changed form state must refuse before a later trusted action.
    pub(crate) fn same_fingerprint(&self, observed: &Self) -> bool {
        self.0 == observed.0
    }
}
impl std::fmt::Debug for Binding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SnapshotBinding([redacted])")
    }
}
/// The measured result is not evidence of a complete DOM, paint or live engine.
#[derive(Debug)]
pub(crate) struct Snapshot {
    pub(crate) content: CappedText,
    pub(crate) stats: SnapshotStats,
}

/// Call only for a result from the expected current target/isolated realm.
/// The namespace was reserved before sending the traversal, including failed traversals.
pub(crate) fn finish(
    source: &str,
    namespace: SnapshotNamespace,
    references: &mut References<Binding>,
) -> Result<Snapshot, SnapshotError> {
    let result = decode(source, namespace, references);
    if result.is_err() {
        // Even a partially decoded/retained graph must not resolve later.
        references.invalidate()?;
    }
    result
}
fn decode(
    source: &str,
    namespace: SnapshotNamespace,
    references: &mut References<Binding>,
) -> Result<Snapshot, SnapshotError> {
    let graph: Graph = serde_json::from_str(source).map_err(SnapshotError::Decode)?;
    complete(graph, namespace, references)
}
fn complete(
    graph: Graph,
    namespace: SnapshotNamespace,
    references: &mut References<Binding>,
) -> Result<Snapshot, SnapshotError> {
    references.check_namespace(namespace)?;
    graph.validate()?;
    let content = render::render(&graph)?;
    let count = retain(&graph, namespace, references)?;
    Ok(Snapshot {
        stats: SnapshotStats {
            nodes: SnapshotMetric::new(graph.visited),
            references: SnapshotMetric::new(count),
            text_bytes: TextByteCount::new(
                u32::try_from(content.text.as_str().len())
                    .map_err(|_overflow| SnapshotError::Graph)?,
            ),
            frames: SnapshotMetric::new(graph.frames),
        },
        content,
    })
}
fn retain(
    graph: &Graph,
    namespace: SnapshotNamespace,
    references: &mut References<Binding>,
) -> Result<u32, SnapshotError> {
    let mut count = 0_u32;
    for node in &graph.nodes.0 {
        if let Node::Element {
            reference: Some(reference),
            fingerprint,
            ..
        } = node
        {
            let expected: ElementRef = namespace.at(count)?;
            if expected != *reference {
                return Err(SnapshotError::Graph);
            }
            references.retain(namespace, reference.clone(), Binding(fingerprint.clone()))?;
            count = count.checked_add(1).ok_or(SnapshotError::Graph)?;
        }
    }
    Ok(count)
}

/// Evaluate data is decoded directly into the bounded graph, not a generic JSON tree.
pub(crate) fn finish_evaluation(
    result: &crate::cdp::requests::Evaluated,
    namespace: SnapshotNamespace,
    references: &mut References<Binding>,
) -> Result<Snapshot, SnapshotError> {
    #[derive(serde::Deserialize)]
    struct ReturnedGraph {
        value: Graph,
    }
    let decoded = result
        .remote_object::<ReturnedGraph>()
        .map_err(SnapshotError::Evaluation)
        .and_then(|result| complete(result.value, namespace, references));
    if decoded.is_err() {
        references.invalidate()?;
    }
    decoded
}
