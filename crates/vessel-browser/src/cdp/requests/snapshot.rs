//! Sealed isolated-world commands, not an arbitrary execution-context selector.
use super::{Evaluated, Method, Request, sealed};
use crate::cdp::identity::{ContextId, FrameId, LoaderId};
use crate::snapshot::Config;
use serde::{Deserialize, Serialize};
const WORLD_NAME: &str = "vessel-browser-snapshot";

#[derive(Debug, Serialize)]
pub(crate) struct FrameTree {}
request!(FrameTree, FrameTreeResult, FrameTree);
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FrameTreeResult {
    pub(crate) frame_tree: FrameRoot,
}
#[derive(Debug, Deserialize)]
pub(crate) struct FrameRoot {
    pub(crate) frame: Frame,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Frame {
    pub(crate) id: FrameId,
    pub(crate) loader_id: LoaderId,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IsolatedWorld {
    frame_id: FrameId,
    world_name: &'static str,
    // The upstream wire spelling deliberately omits the second 's'.
    #[serde(rename = "grantUniveralAccess")]
    universal_access: bool,
}
impl IsolatedWorld {
    pub(crate) const fn new(frame_id: FrameId) -> Self {
        Self {
            frame_id,
            world_name: WORLD_NAME,
            universal_access: false,
        }
    }
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct World {
    pub(crate) execution_context_id: ContextId,
}
request!(IsolatedWorld, World, IsolatedWorld);

/// There is no public source/function/context constructor or generic CDP command tunnel.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SnapshotTraversal {
    function_declaration: &'static str,
    execution_context_id: ContextId,
    arguments: [Argument; 1],
    return_by_value: bool,
    await_promise: bool,
    user_gesture: bool,
}
#[derive(Debug, Serialize)]
struct Argument {
    value: Config,
}
impl SnapshotTraversal {
    pub(crate) const fn new(context: ContextId, config: Config) -> Self {
        Self {
            function_declaration: crate::snapshot::SOURCE,
            execution_context_id: context,
            arguments: [Argument { value: config }],
            return_by_value: true,
            await_promise: false,
            user_gesture: false,
        }
    }
}
impl std::fmt::Debug for SnapshotTraversal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OwnedSnapshotTraversal([private])")
    }
}
request!(SnapshotTraversal, Evaluated, SnapshotTraversal);
